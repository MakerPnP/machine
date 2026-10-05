//! Discovers io boards on the local network and connects to them, see [`ioboard_shared::discovery`].
//!
//! Each io board definition is an entry that is either empty, pending or connected:
//! * pending: a board has been selected for the entry, the entry's socket is bound and the board is being claimed, but
//!   no ergot interface is registered yet.
//! * connected: the board confirmed the claim (its advertisement shows it's claimed by the entry's endpoint), the
//!   ergot interface is registered.
//!
//! From the server's point of view, an advertisement's `claimed_by` is one of [`ClaimedBy`].  Since only one server can
//! run per machine (see [`bind_discovery_socket`]), a claim by this machine's IP address that isn't the entry's current
//! endpoint must have been left by a previous instance of this server, it's released and the board re-claimed.

use std::collections::{HashMap, HashSet};
use std::io::ErrorKind;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;

use ergot::net_stack::NetStackHandle;
use ergot::toolkits::tokio_udp::{RouterStack, register_router_interface};
use ioboard_shared::discovery::{
    self, DISCOVERY_FRAME_SIZE_MAX, DISCOVERY_PORT, DecodeError, Endpoint, IoBoardAdvertisement,
    IoBoardAdvertisementTopic, IoBoardClaim, IoBoardClaimTopic, IoBoardRelease, IoBoardReleaseTopic, SerialNumber,
};
use log::{debug, error, info, warn};
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::broadcast::Receiver;
use tokio::time::{Instant, timeout_at};

use crate::AppEvent;
use crate::config::{ConnectionKind, DiscoveredIoBoard, IO_BOARD_LOCAL_PORT_BASE, IoBoardDefinition};
use crate::ioboard::IOBOARD_TX_BUFFER_SIZE;
use crate::networking::UDP_OVER_ETH_ERGOT_PAYLOAD_SIZE_MAX;

/// How many times a release is sent to a board that hasn't confirmed it, when shutting down.
const RELEASE_ATTEMPTS: u32 = 4;
/// How long to wait for boards to confirm a release, before sending it again.
const RELEASE_RETRY_INTERVAL: Duration = Duration::from_millis(250);

/// Binds the discovery port.
///
/// Fails if another server is already running on this machine, which is relied upon to detect stale claims, see the
/// module documentation.
pub async fn bind_discovery_socket() -> anyhow::Result<UdpSocket> {
    UdpSocket::bind((Ipv4Addr::UNSPECIFIED, DISCOVERY_PORT))
        .await
        .map_err(|e| {
            anyhow::format_err!(
                "Unable to bind the io board discovery port, is another server already running on this machine? port: {}, error: {}",
                DISCOVERY_PORT,
                e
            )
        })
}

pub async fn io_board_discovery(
    stack: RouterStack,
    definitions: Vec<IoBoardDefinition>,
    socket: UdpSocket,
    app_event_rx: Receiver<AppEvent>,
) {
    let mut app_shutdown_handler = Box::pin(crate::app_shutdown_handler(app_event_rx));

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

    // so the boards can be claimed by another server, or this server with a different config, without being reset.
    discovery.release_all(&socket).await;

    info!("io board discovery stopped");
}

/// What an advertisement's `claimed_by` means to this server.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ClaimedBy {
    Unclaimed,
    /// Claimed by the endpoint of the entry the board is assigned to.
    Us,
    /// Claimed by an endpoint on this machine that isn't the entry's endpoint, left by a previous instance of this
    /// server.
    Stale(Endpoint),
    /// Claimed by another server.
    Other(Endpoint),
}

fn classify(claimed_by: Option<Endpoint>, local_endpoint: Option<Endpoint>, local_ip: Option<Ipv4Addr>) -> ClaimedBy {
    match claimed_by {
        None => ClaimedBy::Unclaimed,
        Some(claimed_by) if Some(claimed_by) == local_endpoint => ClaimedBy::Us,
        Some(claimed_by) if Some(Ipv4Addr::from(claimed_by.ip)) == local_ip => ClaimedBy::Stale(claimed_by),
        Some(claimed_by) => ClaimedBy::Other(claimed_by),
    }
}

/// The local IP address used to reach `ip`.  No packets are sent.
fn local_ip_towards(ip: Ipv4Addr) -> Option<Ipv4Addr> {
    let socket = std::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket
        .connect((ip, DISCOVERY_PORT))
        .ok()?;
    match socket.local_addr().ok()? {
        SocketAddr::V4(address) => Some(*address.ip()),
        SocketAddr::V6(_) => None,
    }
}

struct IoBoard {
    serial_number: SerialNumber,
    /// The board's ergot endpoint.
    address: SocketAddrV4,
    /// Our ergot endpoint for this board, sent to the board in claims.
    local_endpoint: Endpoint,
    /// The board's discovery endpoint, the source of its most recent advertisement.
    discovery_address: SocketAddrV4,
    /// From the board's most recent advertisement.
    claimed_by: Option<Endpoint>,
    /// The other server the board is claimed by, used to avoid repeated warnings.
    claimed_by_other: Option<Endpoint>,
}

enum Entry {
    Empty,
    Pending {
        board: IoBoard,
        /// Bound to the entry's port and connected to the board, the ergot interface is registered with it once the
        /// board confirms the claim.
        socket: UdpSocket,
        /// A stale claim is being released, the board is claimed once it is unclaimed.
        releasing_stale: bool,
    },
    Connected {
        board: IoBoard,
        /// The router interface identifier.
        interface: u8,
    },
}

impl Entry {
    fn board(&self) -> Option<&IoBoard> {
        match self {
            Entry::Empty => None,
            Entry::Pending {
                board, ..
            }
            | Entry::Connected {
                board, ..
            } => Some(board),
        }
    }

    fn board_mut(&mut self) -> Option<&mut IoBoard> {
        match self {
            Entry::Empty => None,
            Entry::Pending {
                board, ..
            }
            | Entry::Connected {
                board, ..
            } => Some(board),
        }
    }
}

struct IoBoardDiscovery {
    stack: RouterStack,
    definitions: Vec<IoBoardDefinition>,
    /// One entry per definition.
    entries: Vec<Entry>,
    discovered: HashSet<SerialNumber>,
    /// Unassigned boards claimed by other servers, used to avoid repeated logging.
    skipped: HashMap<SerialNumber, Endpoint>,
    /// `None` to use ephemeral ports, for tests.
    local_port_base: Option<u16>,
}

impl IoBoardDiscovery {
    fn new(stack: RouterStack, definitions: Vec<IoBoardDefinition>, local_port_base: Option<u16>) -> Self {
        Self {
            stack,
            local_port_base,
            entries: definitions
                .iter()
                .map(|_| Entry::Empty)
                .collect(),
            definitions,
            discovered: HashSet::new(),
            skipped: HashMap::new(),
        }
    }

    /// Returns the index of the entry the unassigned board should be assigned to, if any.
    ///
    /// A definition with a matching `Id` takes precedence, regardless of who the board is claimed by.  Otherwise the
    /// first empty `First` entry is used, but only when the board can be claimed by us.
    fn select_entry(&self, serial_number: SerialNumber, claimed_by: ClaimedBy) -> Option<usize> {
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

        if matches!(claimed_by, ClaimedBy::Other(_)) {
            return None;
        }

        self.definitions
            .iter()
            .zip(self.entries.iter())
            .position(|(definition, entry)| {
                matches!(
                    definition.connection,
                    ConnectionKind::Discovered(DiscoveredIoBoard::First)
                ) && matches!(entry, Entry::Empty)
            })
    }

    /// Assigns an unassigned board to an entry, if possible, returning the entry's index.
    async fn assign(&mut self, from: SocketAddrV4, advertisement: &IoBoardAdvertisement) -> Option<usize> {
        let serial_number = advertisement.serial_number;
        let claimed_by = classify(advertisement.claimed_by, None, local_ip_towards(*from.ip()));

        let Some(index) = self.select_entry(serial_number, claimed_by) else {
            if let ClaimedBy::Other(other) = claimed_by {
                if self
                    .skipped
                    .insert(serial_number, other)
                    != Some(other)
                {
                    info!(
                        "Io board is claimed by another server, skipping. serial_number: {}, claimed_by: {}",
                        serial_number,
                        SocketAddrV4::from(other)
                    );
                }
            }
            return None;
        };
        self.skipped.remove(&serial_number);

        let name = &self.definitions[index].name;
        let address = SocketAddrV4::new(*from.ip(), advertisement.ergot_port);
        let local_port = self
            .local_port_base
            .map(|base| base + index as u16)
            .unwrap_or(0);

        let (socket, local_endpoint) = match bind(address, local_port).await {
            Ok(result) => result,
            // e.g. the port is still held by the ergot interface for a previous board, it's released asynchronously.
            Err(e) if e.kind() == ErrorKind::AddrInUse => {
                debug!(
                    "Io board entry port in use, retrying on next advertisement. name: {}, port: {}",
                    name, local_port
                );
                return None;
            }
            Err(e) => {
                error!(
                    "Unable to bind socket for io board. name: {}, serial_number: {}, address: {}, port: {}, error: {}",
                    name, serial_number, address, local_port, e
                );
                return None;
            }
        };

        info!(
            "Selected io board. name: {}, serial_number: {}, address: {}, local_address: {}",
            name,
            serial_number,
            address,
            SocketAddrV4::from(local_endpoint)
        );

        self.entries[index] = Entry::Pending {
            board: IoBoard {
                serial_number,
                address,
                local_endpoint,
                discovery_address: from,
                claimed_by: advertisement.claimed_by,
                claimed_by_other: None,
            },
            socket,
            releasing_stale: false,
        };
        Some(index)
    }

    /// Returns the entry to the empty state, deregistering the ergot interface if connected.
    fn clear(&mut self, index: usize) {
        if let Entry::Connected {
            interface, ..
        } = &self.entries[index]
        {
            let interface = *interface;
            let result = self
                .stack
                .stack()
                .manage_profile(|im| im.deregister_interface(interface));
            if let Err(e) = result {
                warn!(
                    "Unable to deregister io board interface. name: {}, error: {:?}",
                    self.definitions[index].name, e
                );
            }
        }
        self.entries[index] = Entry::Empty;
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
            .entries
            .iter()
            .position(|entry| matches!(entry.board(), Some(board) if board.serial_number == serial_number));

        // the board's address changed, e.g. a new DHCP lease, start over with the new address.
        if let Some(index) = existing {
            let board = self.entries[index].board().unwrap();
            let address = SocketAddrV4::new(*from.ip(), advertisement.ergot_port);
            if board.address != address {
                info!(
                    "Io board address changed, reconnecting. name: {}, serial_number: {}, old: {}, new: {}",
                    self.definitions[index].name, serial_number, board.address, address
                );
                self.clear(index);
            }
        }

        let existing = self
            .entries
            .iter()
            .position(|entry| matches!(entry.board(), Some(board) if board.serial_number == serial_number));
        let index = match existing {
            Some(index) => index,
            None => match self.assign(from, &advertisement).await {
                Some(index) => index,
                None => return,
            },
        };

        let board = self.entries[index].board_mut().unwrap();
        board.discovery_address = from;
        board.claimed_by = advertisement.claimed_by;
        let local_endpoint = board.local_endpoint;
        let claimed_by = classify(
            advertisement.claimed_by,
            Some(local_endpoint),
            Some(local_endpoint.ip.into()),
        );

        match claimed_by {
            ClaimedBy::Unclaimed => {
                if let Entry::Pending {
                    releasing_stale, ..
                } = &mut self.entries[index]
                {
                    *releasing_stale = false;
                }
                // e.g. after we select the board, after the board restarts, or after the claim expired.
                info!(
                    "Claiming io board. name: {}, serial_number: {}",
                    self.definitions[index].name, serial_number
                );
                self.send_claim(socket, index).await;
            }
            ClaimedBy::Us => {
                if let Entry::Pending {
                    releasing_stale: true, ..
                } = &self.entries[index]
                {
                    // the stale claim was by the endpoint we are now using, e.g. this server restarted with the same
                    // config, release it so the board starts a new ergot session with us.
                    self.send_release(socket, index, local_endpoint)
                        .await;
                    return;
                }

                if matches!(self.entries[index], Entry::Pending { .. }) {
                    self.register(index).await;
                }

                // Renew the claim, otherwise the board releases itself after `MISSED_RENEWALS_MAX` advertisements.
                debug!(
                    "Renewing io board claim. name: {}, serial_number: {}",
                    self.definitions[index].name, serial_number
                );
                self.send_claim(socket, index).await;
            }
            ClaimedBy::Stale(stale) => {
                info!(
                    "Releasing stale io board claim. name: {}, serial_number: {}, claimed_by: {}",
                    self.definitions[index].name,
                    serial_number,
                    SocketAddrV4::from(stale)
                );
                if let Entry::Pending {
                    releasing_stale, ..
                } = &mut self.entries[index]
                {
                    *releasing_stale = true;
                }
                self.send_release(socket, index, stale)
                    .await;
            }
            ClaimedBy::Other(other) => self.claimed_by_other(index, other),
        }
    }

    fn claimed_by_other(&mut self, index: usize, other: Endpoint) {
        let name = &self.definitions[index].name;
        let is_first = matches!(
            self.definitions[index].connection,
            ConnectionKind::Discovered(DiscoveredIoBoard::First)
        );
        let connected = matches!(self.entries[index], Entry::Connected { .. });
        let board = self.entries[index].board_mut().unwrap();
        let serial_number = board.serial_number;

        if connected {
            // e.g. our claim expired during a network outage and another server claimed the board.
            warn!(
                "Io board was claimed by another server, disconnecting. name: {}, serial_number: {}, claimed_by: {}",
                name,
                serial_number,
                SocketAddrV4::from(other)
            );
            // a `First` entry can then use another board, an `Id` entry waits for the board to be released.
            self.clear(index);
        } else if is_first {
            info!(
                "Io board was claimed by another server, selecting another board. name: {}, serial_number: {}, claimed_by: {}",
                name,
                serial_number,
                SocketAddrV4::from(other)
            );
            self.clear(index);
        } else if board.claimed_by_other != Some(other) {
            warn!(
                "Io board is claimed by another server, it will be claimed when released. name: {}, serial_number: {}, claimed_by: {}",
                name,
                serial_number,
                SocketAddrV4::from(other)
            );
            board.claimed_by_other = Some(other);
        }
    }

    /// Registers the ergot interface for a pending entry, whose board has confirmed the claim.
    async fn register(&mut self, index: usize) {
        let Entry::Pending {
            board,
            socket,
            ..
        } = std::mem::replace(&mut self.entries[index], Entry::Empty)
        else {
            unreachable!()
        };

        match register_router_interface(
            &self.stack,
            socket,
            UDP_OVER_ETH_ERGOT_PAYLOAD_SIZE_MAX as _,
            IOBOARD_TX_BUFFER_SIZE,
        )
        .await
        {
            Ok(interface) => {
                info!(
                    "Connected to io board. name: {}, serial_number: {}, address: {}, local_address: {}",
                    self.definitions[index].name,
                    board.serial_number,
                    board.address,
                    SocketAddrV4::from(board.local_endpoint)
                );
                self.entries[index] = Entry::Connected {
                    board,
                    interface,
                };
            }
            Err(e) => {
                // the entry is left empty, the board is re-selected on its next advertisement
                error!(
                    "Unable to register io board interface. name: {}, serial_number: {}, error: {:?}",
                    self.definitions[index].name, board.serial_number, e
                );
            }
        }
    }

    async fn send_claim(&self, socket: &UdpSocket, index: usize) {
        let board = self.entries[index].board().unwrap();
        let claim = IoBoardClaim {
            serial_number: board.serial_number,
            server: board.local_endpoint,
        };
        let mut buf = [0u8; DISCOVERY_FRAME_SIZE_MAX];
        let frame = discovery::encode::<IoBoardClaimTopic>(&mut buf, &claim).expect("buffer large enough");
        if let Err(e) = socket
            .send_to(frame, board.discovery_address)
            .await
        {
            warn!(
                "Unable to send claim to io board. name: {}, serial_number: {}, address: {}, error: {}",
                self.definitions[index].name, board.serial_number, board.discovery_address, e
            );
        }
    }

    /// Release the board's claim by `server`.
    async fn send_release(&self, socket: &UdpSocket, index: usize, server: Endpoint) {
        let board = self.entries[index].board().unwrap();
        let release = IoBoardRelease {
            serial_number: board.serial_number,
            server,
        };
        let mut buf = [0u8; DISCOVERY_FRAME_SIZE_MAX];
        let frame = discovery::encode::<IoBoardReleaseTopic>(&mut buf, &release).expect("buffer large enough");
        if let Err(e) = socket
            .send_to(frame, board.discovery_address)
            .await
        {
            warn!(
                "Unable to send release to io board. name: {}, serial_number: {}, address: {}, error: {}",
                self.definitions[index].name, board.serial_number, board.discovery_address, e
            );
        }
    }

    /// Release the claims on all pending and connected boards, waiting for the boards to confirm.
    ///
    /// Boards claimed by another endpoint ignore the release, so they are skipped.
    async fn release_all(&self, socket: &UdpSocket) {
        let mut pending: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| match entry.board() {
                // a pending board may have accepted our claim, but we haven't seen its advertisement yet
                Some(board) if board.claimed_by.is_none() || board.claimed_by == Some(board.local_endpoint) => {
                    Some(index)
                }
                _ => None,
            })
            .collect();

        let mut buf = [0u8; 1500];
        for _ in 0..RELEASE_ATTEMPTS {
            if pending.is_empty() {
                break;
            }

            for &index in &pending {
                let board = self.entries[index].board().unwrap();
                info!(
                    "Releasing io board. name: {}, serial_number: {}",
                    self.definitions[index].name, board.serial_number
                );
                self.send_release(socket, index, board.local_endpoint)
                    .await;
            }

            // wait for advertisements that show the boards are no longer claimed by us
            let deadline = Instant::now() + RELEASE_RETRY_INTERVAL;
            while !pending.is_empty() {
                let Ok(result) = timeout_at(deadline, socket.recv_from(&mut buf)).await else {
                    break;
                };
                let Ok((len, _from)) = result else {
                    continue;
                };
                let Ok(advertisement) = discovery::decode::<IoBoardAdvertisementTopic>(&buf[..len]) else {
                    continue;
                };
                pending.retain(|&index| {
                    let board = self.entries[index].board().unwrap();
                    let released = board.serial_number == advertisement.serial_number
                        && advertisement.claimed_by != Some(board.local_endpoint);
                    if released {
                        info!(
                            "Released io board. name: {}, serial_number: {}",
                            self.definitions[index].name, board.serial_number
                        );
                    }
                    !released
                });
            }
        }

        for index in pending {
            let board = self.entries[index].board().unwrap();
            warn!(
                "Io board did not confirm release, its claim will expire. name: {}, serial_number: {}",
                self.definitions[index].name, board.serial_number
            );
        }
    }
}

/// Binds a socket for an entry and connects it to the board, returning the socket and its local endpoint.
async fn bind(address: SocketAddrV4, local_port: u16) -> std::io::Result<(UdpSocket, Endpoint)> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, local_port)).await?;
    socket.connect(address).await?;
    // after connecting, the local address is the address of the interface used to reach the board
    let SocketAddr::V4(local_address) = socket.local_addr()? else {
        return Err(std::io::Error::other("expected an IPv4 local address"));
    };
    Ok((socket, local_address.into()))
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

    /// Another server, on another machine.
    const OTHER: Endpoint = Endpoint {
        ip: [192, 0, 2, 1],
        port: 8200,
    };
    /// A previous instance of this server, on this machine.
    const STALE: Endpoint = Endpoint {
        ip: [127, 0, 0, 1],
        port: 1,
    };

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

    #[derive(Debug, PartialEq)]
    enum Received {
        Claim(IoBoardClaim),
        Release(IoBoardRelease),
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

        async fn try_recv(&self, wait: Duration) -> Option<Received> {
            let mut buf = [0u8; DISCOVERY_FRAME_SIZE_MAX];
            let (len, _) = timeout(wait, self.discovery.recv_from(&mut buf))
                .await
                .ok()?
                .unwrap();
            match discovery::decode::<IoBoardClaimTopic>(&buf[..len]) {
                Ok(claim) => Some(Received::Claim(claim)),
                Err(_) => Some(Received::Release(
                    discovery::decode::<IoBoardReleaseTopic>(&buf[..len]).unwrap(),
                )),
            }
        }

        async fn try_recv_claim(&self) -> Option<IoBoardClaim> {
            match self
                .try_recv(Duration::from_millis(100))
                .await?
            {
                Received::Claim(claim) => Some(claim),
                received => panic!("expected a claim, got: {:?}", received),
            }
        }

        async fn try_recv_release(&self, wait: Duration) -> Option<IoBoardRelease> {
            match self.try_recv(wait).await? {
                Received::Release(release) => Some(release),
                received => panic!("expected a release, got: {:?}", received),
            }
        }

        async fn send_advertisement(&self, to: SocketAddr, claimed_by: Option<Endpoint>) {
            let mut buf = [0u8; DISCOVERY_FRAME_SIZE_MAX];
            let frame =
                discovery::encode::<IoBoardAdvertisementTopic>(&mut buf, &self.advertisement(claimed_by)).unwrap();
            self.discovery
                .send_to(frame, to)
                .await
                .unwrap();
        }
    }

    #[derive(Debug, PartialEq)]
    enum State {
        Empty,
        Pending(SerialNumber),
        Connected(SerialNumber),
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

        /// Advertise as unclaimed, then confirm the claim, returning the claim.
        async fn connect(&mut self, board: &FakeBoard) -> IoBoardClaim {
            self.advertise(board, None).await;
            let claim = board.try_recv_claim().await.unwrap();
            self.advertise(board, Some(claim.server))
                .await;
            // renewal
            assert_eq!(board.try_recv_claim().await, Some(claim));
            claim
        }

        fn states(&self) -> Vec<State> {
            self.discovery
                .entries
                .iter()
                .map(|entry| match entry {
                    Entry::Empty => State::Empty,
                    Entry::Pending {
                        board, ..
                    } => State::Pending(board.serial_number),
                    Entry::Connected {
                        board, ..
                    } => State::Connected(board.serial_number),
                })
                .collect()
        }
    }

    #[tokio::test]
    async fn interface_is_registered_only_after_the_board_confirms_the_claim() {
        let mut fixture = Fixture::new(vec![first()]).await;
        let board_1 = FakeBoard::new(SERIAL_1).await;

        fixture.advertise(&board_1, None).await;
        let claim = board_1.try_recv_claim().await.unwrap();
        assert_eq!(claim.serial_number, SERIAL_1);
        assert_eq!(claim.server.ip, [127, 0, 0, 1]);
        assert_eq!(fixture.states(), vec![State::Pending(SERIAL_1)]);

        // still unclaimed, e.g. the claim was lost, claim again
        fixture.advertise(&board_1, None).await;
        assert_eq!(board_1.try_recv_claim().await, Some(claim));
        assert_eq!(fixture.states(), vec![State::Pending(SERIAL_1)]);

        // confirmed, the claim is renewed
        fixture
            .advertise(&board_1, Some(claim.server))
            .await;
        assert_eq!(board_1.try_recv_claim().await, Some(claim));
        assert_eq!(fixture.states(), vec![State::Connected(SERIAL_1)]);

        // renewed again
        fixture
            .advertise(&board_1, Some(claim.server))
            .await;
        assert_eq!(board_1.try_recv_claim().await, Some(claim));

        // board restarted, or the claim expired, it's re-claimed
        fixture.advertise(&board_1, None).await;
        assert_eq!(board_1.try_recv_claim().await, Some(claim));
        assert_eq!(fixture.states(), vec![State::Connected(SERIAL_1)]);
    }

    #[tokio::test]
    async fn first_claims_only_one_board() {
        let mut fixture = Fixture::new(vec![first()]).await;
        let board_1 = FakeBoard::new(SERIAL_1).await;
        let board_2 = FakeBoard::new(SERIAL_2).await;

        fixture.connect(&board_1).await;

        fixture.advertise(&board_2, None).await;
        assert!(board_2.try_recv_claim().await.is_none());
        assert!(
            fixture
                .discovery
                .discovered
                .contains(&SERIAL_2)
        );
        assert_eq!(fixture.states(), vec![State::Connected(SERIAL_1)]);
    }

    #[tokio::test]
    async fn first_skips_a_board_claimed_by_another_server() {
        let mut fixture = Fixture::new(vec![first()]).await;
        let board_1 = FakeBoard::new(SERIAL_1).await;
        let board_2 = FakeBoard::new(SERIAL_2).await;

        fixture
            .advertise(&board_1, Some(OTHER))
            .await;
        assert!(board_1.try_recv_claim().await.is_none());
        assert_eq!(fixture.states(), vec![State::Empty]);

        fixture.connect(&board_2).await;
        assert_eq!(fixture.states(), vec![State::Connected(SERIAL_2)]);

        // still skipped
        fixture
            .advertise(&board_1, Some(OTHER))
            .await;
        assert!(board_1.try_recv_claim().await.is_none());
    }

    #[tokio::test]
    async fn pending_first_board_claimed_by_another_server_frees_the_entry() {
        let mut fixture = Fixture::new(vec![first()]).await;
        let board_1 = FakeBoard::new(SERIAL_1).await;
        let board_2 = FakeBoard::new(SERIAL_2).await;

        fixture.advertise(&board_1, None).await;
        board_1.try_recv_claim().await.unwrap();
        assert_eq!(fixture.states(), vec![State::Pending(SERIAL_1)]);

        // another server won
        fixture
            .advertise(&board_1, Some(OTHER))
            .await;
        assert_eq!(fixture.states(), vec![State::Empty]);

        fixture.connect(&board_2).await;
        assert_eq!(fixture.states(), vec![State::Connected(SERIAL_2)]);
    }

    #[tokio::test]
    async fn pending_id_board_claimed_by_another_server_waits_until_unclaimed() {
        let mut fixture = Fixture::new(vec![id(SERIAL_1)]).await;
        let board_1 = FakeBoard::new(SERIAL_1).await;

        fixture
            .advertise(&board_1, Some(OTHER))
            .await;
        assert!(board_1.try_recv_claim().await.is_none());
        assert_eq!(fixture.states(), vec![State::Pending(SERIAL_1)]);

        fixture
            .advertise(&board_1, Some(OTHER))
            .await;
        assert!(board_1.try_recv_claim().await.is_none());

        // released by the other server
        fixture.connect(&board_1).await;
        assert_eq!(fixture.states(), vec![State::Connected(SERIAL_1)]);
    }

    #[tokio::test]
    async fn connected_board_claimed_by_another_server_is_disconnected() {
        let mut fixture = Fixture::new(vec![first()]).await;
        let board_1 = FakeBoard::new(SERIAL_1).await;
        let board_2 = FakeBoard::new(SERIAL_2).await;

        fixture.connect(&board_1).await;
        let interfaces = || {
            fixture
                .discovery
                .stack
                .stack()
                .manage_profile(|im| im.get_nets().len())
        };
        assert_eq!(interfaces(), 1);

        // e.g. our claim expired during a network outage
        fixture
            .advertise(&board_1, Some(OTHER))
            .await;
        assert_eq!(fixture.states(), vec![State::Empty]);
        assert!(board_1.try_recv_claim().await.is_none());
        let interfaces = fixture
            .discovery
            .stack
            .stack()
            .manage_profile(|im| im.get_nets().len());
        assert_eq!(interfaces, 0);

        // the entry can use another board
        fixture.connect(&board_2).await;
        assert_eq!(fixture.states(), vec![State::Connected(SERIAL_2)]);
    }

    #[tokio::test]
    async fn board_address_change_reconnects() {
        let mut fixture = Fixture::new(vec![first()]).await;
        let board_1 = FakeBoard::new(SERIAL_1).await;
        let old_claim = fixture.connect(&board_1).await;

        // same board, new address
        let board_1 = FakeBoard::new(SERIAL_1).await;
        fixture
            .advertise(&board_1, Some(old_claim.server))
            .await;
        assert_eq!(fixture.states(), vec![State::Pending(SERIAL_1)]);
        let Entry::Pending {
            board, ..
        } = &fixture.discovery.entries[0]
        else {
            panic!()
        };
        assert_eq!(
            board.address.port(),
            board_1
                .ergot
                .local_addr()
                .unwrap()
                .port()
        );

        // in tests the entry's port is ephemeral, so the old endpoint is stale, with a fixed port it would be ours.
        assert_eq!(
            board_1
                .try_recv_release(Duration::from_millis(100))
                .await,
            Some(IoBoardRelease {
                serial_number: SERIAL_1,
                server: old_claim.server
            })
        );
        fixture.connect(&board_1).await;
        assert_eq!(fixture.states(), vec![State::Connected(SERIAL_1)]);
    }

    #[tokio::test]
    async fn stale_claim_is_released_then_claimed() {
        let mut fixture = Fixture::new(vec![first()]).await;
        let board_1 = FakeBoard::new(SERIAL_1).await;

        fixture
            .advertise(&board_1, Some(STALE))
            .await;
        assert_eq!(fixture.states(), vec![State::Pending(SERIAL_1)]);
        assert_eq!(
            board_1
                .try_recv_release(Duration::from_millis(100))
                .await,
            Some(IoBoardRelease {
                serial_number: SERIAL_1,
                server: STALE
            })
        );

        fixture.connect(&board_1).await;
        assert_eq!(fixture.states(), vec![State::Connected(SERIAL_1)]);
    }

    #[tokio::test]
    async fn stale_claim_by_the_same_endpoint_is_released_then_claimed() {
        // e.g. this server restarted with the same config, and the same fixed port
        let mut fixture = Fixture::new(vec![first()]).await;
        let board_1 = FakeBoard::new(SERIAL_1).await;

        fixture
            .advertise(&board_1, Some(STALE))
            .await;
        board_1
            .try_recv_release(Duration::from_millis(100))
            .await
            .unwrap();

        // the board still shows the stale claim, which happens to be the entry's endpoint
        let Entry::Pending {
            board, ..
        } = &fixture.discovery.entries[0]
        else {
            panic!()
        };
        let local_endpoint = board.local_endpoint;
        fixture
            .advertise(&board_1, Some(local_endpoint))
            .await;
        assert_eq!(
            board_1
                .try_recv_release(Duration::from_millis(100))
                .await,
            Some(IoBoardRelease {
                serial_number: SERIAL_1,
                server: local_endpoint
            })
        );
        assert_eq!(fixture.states(), vec![State::Pending(SERIAL_1)]);

        fixture.connect(&board_1).await;
        assert_eq!(fixture.states(), vec![State::Connected(SERIAL_1)]);
    }

    #[tokio::test]
    async fn port_in_use_is_retried_on_next_advertisement() {
        // arbitrary, unlikely to be in use
        const BASE: u16 = 48300;
        let mut fixture = Fixture::new(vec![first()]).await;
        fixture.discovery.local_port_base = Some(BASE);
        let board_1 = FakeBoard::new(SERIAL_1).await;

        let blocker = std::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, BASE)).unwrap();
        fixture.advertise(&board_1, None).await;
        assert!(board_1.try_recv_claim().await.is_none());
        assert_eq!(fixture.states(), vec![State::Empty]);

        drop(blocker);
        fixture.advertise(&board_1, None).await;
        assert_eq!(
            board_1
                .try_recv_claim()
                .await
                .unwrap()
                .server
                .port,
            BASE
        );
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
    async fn release_all_releases_claimed_boards() {
        let mut fixture = Fixture::new(vec![first()]).await;
        let board_1 = FakeBoard::new(SERIAL_1).await;
        let claim = fixture.connect(&board_1).await;

        let server_address = fixture.socket.local_addr().unwrap();
        let started_at = Instant::now();
        let ((), release) = tokio::join!(
            fixture
                .discovery
                .release_all(&fixture.socket),
            async {
                let release = board_1
                    .try_recv_release(Duration::from_secs(1))
                    .await
                    .unwrap();
                // the board confirms by advertising that it's unclaimed
                board_1
                    .send_advertisement(server_address, None)
                    .await;
                release
            }
        );

        assert_eq!(release, IoBoardRelease {
            serial_number: SERIAL_1,
            server: claim.server,
        });
        // confirmed, so no retries
        assert!(started_at.elapsed() < RELEASE_RETRY_INTERVAL);
    }

    #[tokio::test]
    async fn release_all_releases_pending_boards() {
        let mut fixture = Fixture::new(vec![first()]).await;
        let board_1 = FakeBoard::new(SERIAL_1).await;
        fixture.advertise(&board_1, None).await;
        let claim = board_1.try_recv_claim().await.unwrap();
        assert_eq!(fixture.states(), vec![State::Pending(SERIAL_1)]);

        let server_address = fixture.socket.local_addr().unwrap();
        let ((), release) = tokio::join!(
            fixture
                .discovery
                .release_all(&fixture.socket),
            async {
                let release = board_1
                    .try_recv_release(Duration::from_secs(1))
                    .await
                    .unwrap();
                board_1
                    .send_advertisement(server_address, None)
                    .await;
                release
            }
        );

        assert_eq!(release.server, claim.server);
    }

    #[tokio::test]
    async fn release_all_retries_then_gives_up_when_unconfirmed() {
        let mut fixture = Fixture::new(vec![first()]).await;
        let board_1 = FakeBoard::new(SERIAL_1).await;
        fixture.connect(&board_1).await;

        fixture
            .discovery
            .release_all(&fixture.socket)
            .await;

        let mut releases = 0;
        while board_1
            .try_recv_release(Duration::from_millis(10))
            .await
            .is_some()
        {
            releases += 1;
        }
        assert_eq!(releases, RELEASE_ATTEMPTS);
    }

    #[tokio::test]
    async fn release_all_skips_boards_claimed_by_another_server() {
        let mut fixture = Fixture::new(vec![id(SERIAL_1)]).await;
        let board_1 = FakeBoard::new(SERIAL_1).await;
        fixture
            .advertise(&board_1, Some(OTHER))
            .await;

        fixture
            .discovery
            .release_all(&fixture.socket)
            .await;

        assert!(
            board_1
                .try_recv_release(Duration::from_millis(100))
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn boards_are_assigned_to_definitions_by_id_regardless_of_discovery_order() {
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
        assert_eq!(fixture.states(), vec![
            State::Pending(SERIAL_1),
            State::Pending(SERIAL_2)
        ]);
    }

    #[tokio::test]
    async fn first_does_not_take_a_board_with_a_matching_id_definition() {
        let mut fixture = Fixture::new(vec![first(), id(SERIAL_2)]).await;
        let board_1 = FakeBoard::new(SERIAL_1).await;
        let board_2 = FakeBoard::new(SERIAL_2).await;
        let board_3 = FakeBoard::new(SERIAL_3).await;

        fixture.advertise(&board_2, None).await;
        assert_eq!(fixture.states(), vec![State::Empty, State::Pending(SERIAL_2)]);

        fixture.advertise(&board_1, None).await;
        fixture.advertise(&board_3, None).await;
        assert_eq!(fixture.states(), vec![
            State::Pending(SERIAL_1),
            State::Pending(SERIAL_2)
        ]);
        assert!(board_1.try_recv_claim().await.is_some());
        assert!(board_2.try_recv_claim().await.is_some());
        assert!(board_3.try_recv_claim().await.is_none());
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

        assert_eq!(fixture.states(), vec![
            State::Pending(SERIAL_2),
            State::Pending(SERIAL_1)
        ]);
    }
}
