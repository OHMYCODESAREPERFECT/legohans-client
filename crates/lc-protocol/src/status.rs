//! Handshake and status ("server list ping") packets.

use crate::codec::{frame_packet, write_string, write_varint};

/// Packet id of the handshake packet (handshaking state).
pub const HANDSHAKE_PACKET_ID: i32 = 0x00;
/// Packet id of the status request and status response (status state).
pub const STATUS_PACKET_ID: i32 = 0x00;
/// Packet id of the ping request and pong response (status state).
pub const PING_PACKET_ID: i32 = 0x01;
/// Handshake "next state" value that selects the status state.
pub const NEXT_STATE_STATUS: i32 = 1;
/// Upper bound for a status response packet (the favicon makes it large).
pub const MAX_STATUS_PACKET_BYTES: usize = 256 * 1024;
/// Upper bound in bytes for the status JSON string.
pub const MAX_STATUS_JSON_BYTES: usize = MAX_STATUS_PACKET_BYTES;

/// Build the handshake packet that switches a connection to the status state.
pub fn handshake_packet(protocol_version: i32, host: &str, port: u16) -> Vec<u8> {
    let mut body = Vec::new();
    write_varint(&mut body, protocol_version);
    write_string(&mut body, host);
    body.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut body, NEXT_STATE_STATUS);
    frame_packet(HANDSHAKE_PACKET_ID, &body)
}

/// Build the (empty) status request packet.
pub fn status_request_packet() -> Vec<u8> {
    frame_packet(STATUS_PACKET_ID, &[])
}

/// Build a ping request carrying `payload`.
pub fn ping_packet(payload: i64) -> Vec<u8> {
    frame_packet(PING_PACKET_ID, &payload.to_be_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PROTOCOL_VERSION;
    use crate::codec::{read_packet, read_string, read_varint};

    #[test]
    fn handshake_contains_expected_fields() {
        let bytes = handshake_packet(PROTOCOL_VERSION, "example.org", 25565);
        let (id, body) = read_packet(&mut bytes.as_slice(), 256).unwrap();
        assert_eq!(id, HANDSHAKE_PACKET_ID);
        let mut cur = body.as_slice();
        assert_eq!(read_varint(&mut cur).unwrap(), 774);
        assert_eq!(read_string(&mut cur, 255).unwrap(), "example.org");
        assert_eq!(&cur[..2], &25565u16.to_be_bytes());
        cur = &cur[2..];
        assert_eq!(read_varint(&mut cur).unwrap(), NEXT_STATE_STATUS);
        assert!(cur.is_empty());
    }

    #[test]
    fn status_request_is_empty_packet() {
        assert_eq!(status_request_packet(), [1, 0]);
    }

    #[test]
    fn ping_carries_payload() {
        let bytes = ping_packet(0x0102_0304_0506_0708);
        let (id, body) = read_packet(&mut bytes.as_slice(), 64).unwrap();
        assert_eq!(id, PING_PACKET_ID);
        assert_eq!(body, 0x0102_0304_0506_0708i64.to_be_bytes());
    }
}
