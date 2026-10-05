//! Discovers io boards on the local network and connects to them, see [`ioboard_shared::discovery`].

use std::collections::HashSet;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use ergot::toolkits::tokio_udp::{RouterStack, register_router_interface};
use ioboard_shared::discovery::{
    self, DISCOVERY_FRAME_SIZE_MAX, DISCOVERY_PORT, DecodeError, Endpoint, IoBoardAdvertisement,
    IoBoardAdvertisementTopic, IoBoardClaim, IoBoardClaimTopic, SerialNumber,
};
use log::{debug, error, info, warn};
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::broadcast::Receiver;

use crate::AppEvent;
use crate::config::{ConnectionKind, DiscoveredIoBoard, IO_BOARD_LOCAL_PORT_BASE, IoBoardDefinition};
use crate::ioboard::IOBOARD_TX_BUFFER_SIZE;
use crate::networking::UDP_OVER_ETH_ERGOT_PAYLOAD_SIZE_MAX;

struct ConnectedIoBoard {
    serial_number: SerialNumber,
    /// The board's ergot endpoint.
    address: SocketAddrV4,
    /// Our ergot endpoint for this board, sent to the board in claims.
    local_endpoint: Endpoint,
    /// The endpoint the board is claimed by, when it is not ours, used to avoid repeated warnings.
    claimed_elsewhere: Option<Endpoint>,
}

pub async fn io_board_discovery(
    stack: RouterStack,
    definitions: Vec<IoBoardDefinition>,
    app_event_rx: Receiver<AppEvent>,
) {
    let mut app_shutdown_handler = Box::pin(crate::app_shutdown_handler(app_event_rx));

    let socket = match UdpSocket::bind((Ipv4Addr::UNSPECIFIED, DISCOVERY_PORT)).await {
        Ok(socket) => socket,
        Err(e) => {
            error!(
                "Unable to create io board discovery socket, io boards will not be connected. port: {}, error: {}",
                DISCOVERY_PORT, e
            );
            return;
        }
    };

    info!(
        "Listening for io board advertisements. port: {}, io boards: {:?}",
        DISCOVERY_PORT, definitions
    );

    let mut discovery = IoBoardDiscovery::new(stack, definitions, Some(IO_BOARD_LOCAL_PORT_BASE));
    let mut incompatible: HashSet<SocketAddr> = HashSet::new();

    let mut buf = [0u8; 1500];
    loop {
        let (len, from) = select! {
            result = socket.recv_from(&mut buf) => match result {
                Ok(result) => result,
                Err(e) => {
                    // e.g. on Windows, an ICMP port unreachable for a previously sent claim is reported here.
                    debug!("Io board discovery receive error. error: {}", e);
                    continue
                }
            },
            _ = &mut app_shutdown_handler => break,
        };

        let advertisement = match discovery::decode::<IoBoardAdvertisementTopic>(&buf[..len]) {
            Ok(advertisement) => advertisement,
            Err(DecodeError::UnknownKey) => {
                if incompatible.insert(from) {
                    warn!(
                        "Ignoring discovery packet with an unknown key, the io board firmware may be incompatible. address: {}",
                        from
                    );
                }
                continue;
            }
            Err(e) => {
                debug!("Ignoring invalid discovery packet. address: {}, error: {:?}", from, e);
                continue;
            }
        };
        let SocketAddr::V4(from) = from else {
            continue;
        };

        discovery
            .handle_advertisement(&socket, from, advertisement)
            .await;
    }

    info!("io board discovery stopped");
}

struct IoBoardDiscovery {
    stack: RouterStack,
    definitions: Vec<IoBoardDefinition>,
    /// One entry per definition, `Some` once a board has been connected for that definition.
    connected: Vec<Option<ConnectedIoBoard>>,
    discovered: HashSet<SerialNumber>,
    /// `None` to use ephemeral ports, for tests.
    local_port_base: Option<u16>,
}

impl IoBoardDiscovery {
    fn new(stack: RouterStack, definitions: Vec<IoBoardDefinition>, local_port_base: Option<u16>) -> Self {
        Self {
            stack,
            local_port_base,
            connected: definitions
                .iter()
                .map(|_| None)
                .collect(),
            definitions,
            discovered: HashSet::new(),
        }
    }

    /// Returns the index of the definition the board should be connected for, if any.
    ///
    /// A definition with a matching `Id` takes precedence, otherwise the first unconnected `First` definition is used.
    fn select_definition(&self, serial_number: SerialNumber) -> Option<usize> {
        let by_id = self
            .definitions
            .iter()
            .position(|definition| match &definition.connection {
                ConnectionKind::Discovered(DiscoveredIoBoard::Id(id)) => id.0 == serial_number,
                ConnectionKind::Discovered(DiscoveredIoBoard::First) => false,
            });
        if by_id.is_some() {
            return by_id;
        }

        self.definitions
            .iter()
            .zip(self.connected.iter())
            .position(|(definition, connected)| {
                matches!(
                    definition.connection,
                    ConnectionKind::Discovered(DiscoveredIoBoard::First)
                ) && connected.is_none()
            })
    }

    async fn handle_advertisement(
        &mut self,
        socket: &UdpSocket,
        from: SocketAddrV4,
        advertisement: IoBoardAdvertisement,
    ) {
        let serial_number = advertisement.serial_number;

        if self.discovered.insert(serial_number) {
            info!(
                "Discovered io board. serial_number: {}, address: {}",
                serial_number,
                from.ip()
            );
        }

        let existing = self
            .connected
            .iter()
            .position(|connected| matches!(connected, Some(board) if board.serial_number == serial_number));

        let index = match existing {
            Some(index) => index,
            None => {
                let Some(index) = self.select_definition(serial_number) else {
                    return;
                };

                let address = SocketAddrV4::new(*from.ip(), advertisement.ergot_port);
                let local_port = self
                    .local_port_base
                    .map(|base| base + index as u16)
                    .unwrap_or(0);
                match connect(&self.stack, serial_number, address, local_port).await {
                    Ok(board) => {
                        info!(
                            "Connected to io board. name: {}, serial_number: {}, address: {}, local_address: {}",
                            self.definitions[index].name,
                            serial_number,
                            address,
                            SocketAddrV4::from(board.local_endpoint)
                        );
                        self.connected[index] = Some(board);
                    }
                    Err(e) => {
                        error!(
                            "Unable to connect to io board. name: {}, serial_number: {}, address: {}, error: {:?}",
                            self.definitions[index].name, serial_number, address, e
                        );
                        return;
                    }
                }
                index
            }
        };
        let name = &self.definitions[index].name;
        let board = self.connected[index].as_mut().unwrap();

        if board.address.ip() != from.ip() || board.address.port() != advertisement.ergot_port {
            // FUTURE handle this by replacing the ergot interface
            warn!(
                "Io board address changed, restart the server to reconnect. name: {}, serial_number: {}, old: {}, new: {}:{}",
                self.definitions[index].name,
                serial_number,
                board.address,
                from.ip(),
                advertisement.ergot_port
            );
            return;
        }

        match advertisement.claimed_by {
            Some(claimed_by) if claimed_by == board.local_endpoint => {
                board.claimed_elsewhere = None;
                return;
            }
            Some(claimed_by) => {
                // boards only accept the first claim, until they are reset.
                if board.claimed_elsewhere != Some(claimed_by) {
                    warn!(
                        "Io board is claimed by another endpoint, reset the io board to connect. name: {}, serial_number: {}, claimed_by: {}, local_address: {}",
                        name,
                        serial_number,
                        SocketAddrV4::from(claimed_by),
                        SocketAddrV4::from(board.local_endpoint)
                    );
                    board.claimed_elsewhere = Some(claimed_by);
                }
                return;
            }
            None => {}
        }

        // Claim the unclaimed board, e.g. after we connect, or after the board restarts.
        {
            info!("Claiming io board. name: {}, serial_number: {}", name, serial_number);
            let claim = IoBoardClaim {
                serial_number,
                server: board.local_endpoint,
            };
            let mut buf = [0u8; DISCOVERY_FRAME_SIZE_MAX];
            let frame = discovery::encode::<IoBoardClaimTopic>(&mut buf, &claim).expect("buffer large enough");
            if let Err(e) = socket.send_to(frame, from).await {
                warn!(
                    "Unable to send claim to io board. name: {}, serial_number: {}, address: {}, error: {}",
                    self.definitions[index].name, serial_number, from, e
                );
            }
        }
    }
}

async fn connect(
    stack: &RouterStack,
    serial_number: SerialNumber,
    address: SocketAddrV4,
    local_port: u16,
) -> anyhow::Result<ConnectedIoBoard> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, local_port)).await?;
    socket.connect(address).await?;
    // after connecting, the local address is the address of the interface used to reach the board
    let SocketAddr::V4(local_address) = socket.local_addr()? else {
        anyhow::bail!("Expected an IPv4 local address")
    };

    register_router_interface(
        stack,
        socket,
        UDP_OVER_ETH_ERGOT_PAYLOAD_SIZE_MAX as _,
        IOBOARD_TX_BUFFER_SIZE,
    )
    .await
    .map_err(|e| anyhow::format_err!("Unable to register router interface. error: {:?}", e))?;

    Ok(ConnectedIoBoard {
        serial_number,
        address,
        local_endpoint: local_address.into(),
        claimed_elsewhere: None,
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::timeout;

    use super::*;
    use crate::config::IoBoardId;

    const SERIAL_1: SerialNumber = SerialNumber([1; 12]);
    const SERIAL_2: SerialNumber = SerialNumber([2; 12]);
    const SERIAL_3: SerialNumber = SerialNumber([3; 12]);

    fn first() -> IoBoardDefinition {
        IoBoardDefinition {
            name: "first".to_string(),
            connection: ConnectionKind::Discovered(DiscoveredIoBoard::First),
        }
    }

    fn id(serial_number: SerialNumber) -> IoBoardDefinition {
        IoBoardDefinition {
            name: serial_number.to_string(),
            connection: ConnectionKind::Discovered(DiscoveredIoBoard::Id(IoBoardId(serial_number))),
        }
    }

    struct FakeBoard {
        serial_number: SerialNumber,
        discovery: UdpSocket,
        ergot: UdpSocket,
    }

    impl FakeBoard {
        async fn new(serial_number: SerialNumber) -> Self {
            Self {
                serial_number,
                discovery: UdpSocket::bind("127.0.0.1:0")
                    .await
                    .unwrap(),
                ergot: UdpSocket::bind("127.0.0.1:0")
                    .await
                    .unwrap(),
            }
        }

        fn address(&self) -> SocketAddrV4 {
            let SocketAddr::V4(address) = self.discovery.local_addr().unwrap() else {
                panic!()
            };
            address
        }

        fn advertisement(&self, claimed_by: Option<Endpoint>) -> IoBoardAdvertisement {
            IoBoardAdvertisement {
                serial_number: self.serial_number,
                ergot_port: self.ergot.local_addr().unwrap().port(),
                claimed_by,
            }
        }

        async fn try_recv_claim(&self) -> Option<IoBoardClaim> {
            let mut buf = [0u8; DISCOVERY_FRAME_SIZE_MAX];
            let (len, _) = timeout(Duration::from_millis(100), self.discovery.recv_from(&mut buf))
                .await
                .ok()?
                .unwrap();
            Some(discovery::decode::<IoBoardClaimTopic>(&buf[..len]).unwrap())
        }
    }

    struct Fixture {
        discovery: IoBoardDiscovery,
        socket: UdpSocket,
    }

    impl Fixture {
        async fn new(definitions: Vec<IoBoardDefinition>) -> Self {
            Self {
                discovery: IoBoardDiscovery::new(RouterStack::new(), definitions, None),
                socket: UdpSocket::bind("127.0.0.1:0")
                    .await
                    .unwrap(),
            }
        }

        async fn advertise(&mut self, board: &FakeBoard, claimed_by: Option<Endpoint>) {
            self.discovery
                .handle_advertisement(&self.socket, board.address(), board.advertisement(claimed_by))
                .await;
        }

        fn connected_serial_numbers(&self) -> Vec<Option<SerialNumber>> {
            self.discovery
                .connected
                .iter()
                .map(|connected| {
                    connected
                        .as_ref()
                        .map(|board| board.serial_number)
                })
                .collect()
        }
    }

    #[tokio::test]
    async fn first_claims_only_the_first_board() {
        let mut fixture = Fixture::new(vec![first()]).await;
        let board_1 = FakeBoard::new(SERIAL_1).await;
        let board_2 = FakeBoard::new(SERIAL_2).await;

        fixture.advertise(&board_1, None).await;
        let claim = board_1.try_recv_claim().await.unwrap();
        assert_eq!(claim.serial_number, SERIAL_1);
        assert_eq!(
            claim.server,
            fixture.discovery.connected[0]
                .as_ref()
                .unwrap()
                .local_endpoint
        );
        assert_eq!(claim.server.ip, [127, 0, 0, 1]);

        // already claimed by us, no new claim
        fixture
            .advertise(&board_1, Some(claim.server))
            .await;
        assert!(board_1.try_recv_claim().await.is_none());

        // board restarted, it's re-claimed
        fixture.advertise(&board_1, None).await;
        assert_eq!(board_1.try_recv_claim().await, Some(claim));

        // second board is discovered, but not connected
        fixture.advertise(&board_2, None).await;
        assert!(board_2.try_recv_claim().await.is_none());
        assert!(
            fixture
                .discovery
                .discovered
                .contains(&SERIAL_2)
        );
        assert_eq!(fixture.connected_serial_numbers(), vec![Some(SERIAL_1)]);
    }

    #[tokio::test]
    async fn boards_are_connected_to_definitions_by_id_regardless_of_discovery_order() {
        let mut fixture = Fixture::new(vec![id(SERIAL_1), id(SERIAL_2)]).await;
        let board_1 = FakeBoard::new(SERIAL_1).await;
        let board_2 = FakeBoard::new(SERIAL_2).await;
        let board_3 = FakeBoard::new(SERIAL_3).await;

        fixture.advertise(&board_3, None).await;
        fixture.advertise(&board_2, None).await;
        fixture.advertise(&board_1, None).await;

        assert!(board_3.try_recv_claim().await.is_none());
        assert_eq!(
            board_2
                .try_recv_claim()
                .await
                .unwrap()
                .serial_number,
            SERIAL_2
        );
        assert_eq!(
            board_1
                .try_recv_claim()
                .await
                .unwrap()
                .serial_number,
            SERIAL_1
        );
        assert_eq!(fixture.connected_serial_numbers(), vec![Some(SERIAL_1), Some(SERIAL_2)]);
    }

    #[tokio::test]
    async fn first_does_not_take_a_board_with_a_matching_id_definition() {
        let mut fixture = Fixture::new(vec![first(), id(SERIAL_2)]).await;
        let board_1 = FakeBoard::new(SERIAL_1).await;
        let board_2 = FakeBoard::new(SERIAL_2).await;
        let board_3 = FakeBoard::new(SERIAL_3).await;

        fixture.advertise(&board_2, None).await;
        assert_eq!(fixture.connected_serial_numbers(), vec![None, Some(SERIAL_2)]);

        fixture.advertise(&board_1, None).await;
        fixture.advertise(&board_3, None).await;
        assert_eq!(fixture.connected_serial_numbers(), vec![Some(SERIAL_1), Some(SERIAL_2)]);
        assert!(board_1.try_recv_claim().await.is_some());
        assert!(board_2.try_recv_claim().await.is_some());
        assert!(board_3.try_recv_claim().await.is_none());
    }

    #[tokio::test]
    async fn board_claimed_by_another_endpoint_is_not_claimed() {
        let mut fixture = Fixture::new(vec![first()]).await;
        let board_1 = FakeBoard::new(SERIAL_1).await;
        let elsewhere = Endpoint {
            ip: [127, 0, 0, 1],
            port: 1,
        };

        fixture
            .advertise(&board_1, Some(elsewhere))
            .await;
        assert!(board_1.try_recv_claim().await.is_none());
        assert_eq!(fixture.connected_serial_numbers(), vec![Some(SERIAL_1)]);

        // board was reset, it's now unclaimed
        fixture.advertise(&board_1, None).await;
        assert!(board_1.try_recv_claim().await.is_some());
    }

    #[tokio::test]
    async fn local_port_is_fixed_per_definition() {
        // arbitrary, unlikely to be in use
        const BASE: u16 = 48200;
        let mut fixture = Fixture::new(vec![id(SERIAL_1), id(SERIAL_2)]).await;
        fixture.discovery.local_port_base = Some(BASE);
        let board_2 = FakeBoard::new(SERIAL_2).await;

        fixture.advertise(&board_2, None).await;
        assert_eq!(
            board_2
                .try_recv_claim()
                .await
                .unwrap()
                .server
                .port,
            BASE + 1
        );
    }

    #[tokio::test]
    async fn multiple_first_definitions_take_boards_in_discovery_order() {
        let mut fixture = Fixture::new(vec![first(), first()]).await;
        let board_1 = FakeBoard::new(SERIAL_1).await;
        let board_2 = FakeBoard::new(SERIAL_2).await;

        fixture.advertise(&board_2, None).await;
        fixture.advertise(&board_1, None).await;
        // re-advertising doesn't use up another definition
        fixture.advertise(&board_2, None).await;

        assert_eq!(fixture.connected_serial_numbers(), vec![Some(SERIAL_2), Some(SERIAL_1)]);
    }
}
