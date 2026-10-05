//! IO board discovery protocol.
//!
//! IO boards periodically broadcast an [`IoBoardAdvertisement`] to [`DISCOVERY_PORT`] on the local network.
//! A server listening on that port decides which boards to connect to, creates an ergot UDP interface for each one
//! and then sends a unicast [`IoBoardClaim`] back to the board's discovery socket telling the board where the server's
//! ergot interface is.  The board then points its own ergot interface at that endpoint.
//!
//! A board only accepts a claim when it is unclaimed, it stays claimed by that server endpoint until the server sends
//! an [`IoBoardRelease`] (e.g. when the server shuts down), the claim expires, or the board is reset.  Boards keep
//! advertising after being claimed (with `claimed_by` set), so a server can tell whether a board is unclaimed, claimed
//! by it, or claimed by another endpoint.
//!
//! The server renews its claim by sending the claim again in reply to each advertisement that shows the board is
//! claimed by it.  When [`MISSED_RENEWALS_MAX`] advertisements in a row are not answered by a renewal, e.g. because the
//! server crashed, the claim expires and the board releases itself.  See [`ClaimState`].
//!
//! Servers use a fixed ergot endpoint per board, so that a server that restarts before the claim expires is still the
//! endpoint the board is claimed by.
//!
//! Messages are encoded as ergot topic frames (header + topic key + postcard body) but are sent over a plain UDP socket,
//! outside of any ergot net stack, so that the receiver can learn the sender's IP address from the datagram.  Since the
//! topic key is derived from the message schema, incompatible versions of the protocol are rejected by
//! [`decode`] with [`DecodeError::UnknownKey`].

use core::fmt;
use core::str::FromStr;

use ergot::traits::{Schema, Topic};
use ergot::wire_frames::{de_frame, encode_frame_ty};
use ergot::{Address, AnyAllAppendix, FrameKind, HeaderSeq, Key, topic};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// The UDP port used by both the server and the io boards for discovery traffic.
pub const DISCOVERY_PORT: u16 = 8100;

/// How often an io board broadcasts an advertisement.
pub const ADVERTISEMENT_INTERVAL_MS: u64 = 1000;

/// Large enough for any discovery message.
pub const DISCOVERY_FRAME_SIZE_MAX: usize = 128;

/// Uniquely identifies an io board, derived from the MCU's 96-bit unique device ID.
#[derive(Schema, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SerialNumber(pub [u8; 12]);

impl fmt::Display for SerialNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{:02X}", byte)?;
        }
        Ok(())
    }
}

impl fmt::Debug for SerialNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SerialNumber({})", self)
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for SerialNumber {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "{=[u8]:02X}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseSerialNumberError;

impl fmt::Display for ParseSerialNumberError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("serial number must be 24 hex digits")
    }
}

impl FromStr for SerialNumber {
    type Err = ParseSerialNumberError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.as_bytes();
        if s.len() != 24 {
            return Err(ParseSerialNumberError);
        }
        let mut bytes = [0u8; 12];
        for (byte, pair) in bytes.iter_mut().zip(s.chunks_exact(2)) {
            let pair = core::str::from_utf8(pair).map_err(|_| ParseSerialNumberError)?;
            *byte = u8::from_str_radix(pair, 16).map_err(|_| ParseSerialNumberError)?;
        }
        Ok(Self(bytes))
    }
}

/// An IPv4 UDP endpoint.
#[derive(Schema, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Endpoint {
    pub ip: [u8; 4],
    pub port: u16,
}

impl From<core::net::SocketAddrV4> for Endpoint {
    fn from(value: core::net::SocketAddrV4) -> Self {
        Self {
            ip: value.ip().octets(),
            port: value.port(),
        }
    }
}

impl From<Endpoint> for core::net::SocketAddrV4 {
    fn from(value: Endpoint) -> Self {
        core::net::SocketAddrV4::new(value.ip.into(), value.port)
    }
}

/// Broadcast by io boards.
#[derive(Schema, Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct IoBoardAdvertisement {
    pub serial_number: SerialNumber,
    /// The UDP port of the board's ergot interface.  The IP address is the source address of the advertisement.
    pub ergot_port: u16,
    /// The server ergot endpoint the board is currently sending to, if it has been claimed.
    pub claimed_by: Option<Endpoint>,
}

/// Sent by a server, to the source address of an advertisement, to connect to a board.
#[derive(Schema, Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct IoBoardClaim {
    /// Must match the board's serial number, otherwise the claim is ignored.
    pub serial_number: SerialNumber,
    /// The server's ergot endpoint for this board.
    pub server: Endpoint,
}

topic!(
    IoBoardAdvertisementTopic,
    IoBoardAdvertisement,
    "ioboard/discovery/advertisement"
);
topic!(IoBoardClaimTopic, IoBoardClaim, "ioboard/discovery/claim");

/// Sent by a server, to the board's discovery socket, to release a claim, e.g. when the server shuts down.
///
/// The board replies with an advertisement, which has `claimed_by` set to `None` once the board is released.
#[derive(Schema, Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct IoBoardRelease {
    /// Must match the board's serial number, otherwise the release is ignored.
    pub serial_number: SerialNumber,
    /// Must match the endpoint the board is claimed by, otherwise the release is ignored.
    pub server: Endpoint,
}

topic!(IoBoardReleaseTopic, IoBoardRelease, "ioboard/discovery/release");

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum EncodeError {
    BufferTooSmall,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DecodeError {
    /// Not an ergot topic frame.
    InvalidFrame,
    /// A topic frame, but for a different topic, or an incompatible version of this one.
    UnknownKey,
    /// The body could not be deserialized.
    InvalidBody,
}

/// Encode `msg` as an ergot topic frame into `buf`, returning the used part of `buf`.
pub fn encode<'a, T>(buf: &'a mut [u8], msg: &T::Message) -> Result<&'a mut [u8], EncodeError>
where
    T: Topic,
    T::Message: Serialize + Sized,
{
    let hdr = HeaderSeq {
        src: Address::unknown(),
        dst: Address {
            network_id: 0,
            node_id: 255,
            port_id: 255,
        },
        any_all: Some(AnyAllAppendix {
            key: Key(T::TOPIC_KEY.to_bytes()),
            nash: None,
        }),
        seq_no: 0,
        kind: FrameKind::TOPIC_MSG,
        ttl: 1,
    };

    encode_frame_ty(postcard::ser_flavors::Slice::new(buf), &hdr, msg).map_err(|_| EncodeError::BufferTooSmall)
}

/// Decode an ergot topic frame for topic `T`.
pub fn decode<T>(data: &[u8]) -> Result<T::Message, DecodeError>
where
    T: Topic,
    T::Message: DeserializeOwned + Sized,
{
    let frame = de_frame(data).ok_or(DecodeError::InvalidFrame)?;
    if frame.hdr.kind != FrameKind::TOPIC_MSG {
        return Err(DecodeError::InvalidFrame);
    }
    let appendix = frame
        .hdr
        .any_all
        .ok_or(DecodeError::InvalidFrame)?;
    if appendix.key != Key(T::TOPIC_KEY.to_bytes()) {
        return Err(DecodeError::UnknownKey);
    }
    let body = frame
        .body
        .map_err(|_| DecodeError::InvalidFrame)?;
    postcard::from_bytes(body).map_err(|_| DecodeError::InvalidBody)
}

/// The number of advertisements in a row, sent while claimed, that are not answered by a renewal, after which the
/// claim expires.
///
/// Counted in advertisements, rather than time, so that [`ADVERTISEMENT_INTERVAL_MS`] can be changed without
/// affecting how many lost packets are tolerated.
pub const MISSED_RENEWALS_MAX: u8 = 5;

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ClaimResult {
    /// The board was unclaimed, it's now claimed.
    Claimed,
    /// The board was already claimed by the same endpoint, the claim has been renewed.
    Renewed,
    /// The board is claimed by another endpoint, the claim was ignored.
    ClaimedByOther(Endpoint),
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ReleaseResult {
    Released,
    /// The board was not claimed, e.g. a retry after the reply to an earlier release was lost.
    NotClaimed,
    /// The board is claimed by another endpoint, the release was ignored.
    ClaimedByOther(Endpoint),
}

/// The board side of the claim protocol.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ClaimState {
    claimed_by: Option<Endpoint>,
    missed_renewals: u8,
}

impl ClaimState {
    pub const fn new() -> Self {
        Self {
            claimed_by: None,
            missed_renewals: 0,
        }
    }

    pub fn claimed_by(&self) -> Option<Endpoint> {
        self.claimed_by
    }

    pub fn claim(&mut self, server: Endpoint) -> ClaimResult {
        match self.claimed_by {
            None => {
                self.claimed_by = Some(server);
                self.missed_renewals = 0;
                ClaimResult::Claimed
            }
            Some(claimed_by) if claimed_by == server => {
                self.missed_renewals = 0;
                ClaimResult::Renewed
            }
            Some(claimed_by) => ClaimResult::ClaimedByOther(claimed_by),
        }
    }

    pub fn release(&mut self, server: Endpoint) -> ReleaseResult {
        match self.claimed_by {
            Some(claimed_by) if claimed_by == server => {
                self.claimed_by = None;
                ReleaseResult::Released
            }
            None => ReleaseResult::NotClaimed,
            Some(claimed_by) => ReleaseResult::ClaimedByOther(claimed_by),
        }
    }

    /// Call before sending each periodic advertisement.
    ///
    /// Returns the endpoint the board was claimed by, if the claim expired, in which case the board is now unclaimed.
    pub fn advertising(&mut self) -> Option<Endpoint> {
        let claimed_by = self.claimed_by?;

        if self.missed_renewals >= MISSED_RENEWALS_MAX {
            self.claimed_by = None;
            return Some(claimed_by);
        }

        // reset by a renewal, in reply to this advertisement
        self.missed_renewals += 1;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVER_1: Endpoint = Endpoint {
        ip: [192, 168, 1, 2],
        port: 8200,
    };
    const SERVER_2: Endpoint = Endpoint {
        ip: [192, 168, 1, 3],
        port: 8200,
    };

    #[test]
    fn claim_and_release() {
        let mut state = ClaimState::new();
        assert_eq!(state.release(SERVER_1), ReleaseResult::NotClaimed);

        assert_eq!(state.claim(SERVER_1), ClaimResult::Claimed);
        assert_eq!(state.claimed_by(), Some(SERVER_1));
        assert_eq!(state.claim(SERVER_1), ClaimResult::Renewed);

        assert_eq!(state.claim(SERVER_2), ClaimResult::ClaimedByOther(SERVER_1));
        assert_eq!(state.release(SERVER_2), ReleaseResult::ClaimedByOther(SERVER_1));
        assert_eq!(state.claimed_by(), Some(SERVER_1));

        assert_eq!(state.release(SERVER_1), ReleaseResult::Released);
        assert_eq!(state.claimed_by(), None);

        assert_eq!(state.claim(SERVER_2), ClaimResult::Claimed);
    }

    #[test]
    fn claim_expires_after_missed_renewals() {
        let mut state = ClaimState::new();
        // unclaimed boards never expire
        assert_eq!(state.advertising(), None);

        state.claim(SERVER_1);
        for _ in 0..MISSED_RENEWALS_MAX {
            assert_eq!(state.advertising(), None);
        }
        assert_eq!(state.advertising(), Some(SERVER_1));
        assert_eq!(state.claimed_by(), None);
        assert_eq!(state.advertising(), None);
    }

    #[test]
    fn renewal_resets_missed_renewals() {
        let mut state = ClaimState::new();
        state.claim(SERVER_1);

        for _ in 0..3 * MISSED_RENEWALS_MAX {
            assert_eq!(state.advertising(), None);
            assert_eq!(state.claim(SERVER_1), ClaimResult::Renewed);
        }

        // a claim from another server is not a renewal
        for _ in 0..MISSED_RENEWALS_MAX {
            assert_eq!(state.advertising(), None);
            state.claim(SERVER_2);
        }
        assert_eq!(state.advertising(), Some(SERVER_1));
    }

    const SERIAL: SerialNumber = SerialNumber([0x2C, 0x00, 0x24, 0x00, 0x08, 0x51, 0x33, 0x32, 0x34, 0x36, 0x38, 0xFF]);

    #[test]
    fn serial_number_round_trip() {
        let text = std::format!("{}", SERIAL);
        assert_eq!(text, "2C00240008513332343638FF");
        assert_eq!(text.parse::<SerialNumber>(), Ok(SERIAL));
        assert_eq!("2c00240008513332343638ff".parse::<SerialNumber>(), Ok(SERIAL));
        assert!(
            "2C0024"
                .parse::<SerialNumber>()
                .is_err()
        );
        assert!(
            "ZZ00240008513332343638FF"
                .parse::<SerialNumber>()
                .is_err()
        );
    }

    #[test]
    fn advertisement_round_trip() {
        let advertisement = IoBoardAdvertisement {
            serial_number: SERIAL,
            ergot_port: 8000,
            claimed_by: Some(Endpoint {
                ip: [192, 168, 1, 2],
                port: 54321,
            }),
        };
        let mut buf = [0u8; DISCOVERY_FRAME_SIZE_MAX];
        let used = encode::<IoBoardAdvertisementTopic>(&mut buf, &advertisement).unwrap();

        assert_eq!(decode::<IoBoardAdvertisementTopic>(used), Ok(advertisement));
        assert_eq!(decode::<IoBoardClaimTopic>(used), Err(DecodeError::UnknownKey));
        assert_eq!(decode::<IoBoardAdvertisementTopic>(&[]), Err(DecodeError::InvalidFrame));
    }
}
