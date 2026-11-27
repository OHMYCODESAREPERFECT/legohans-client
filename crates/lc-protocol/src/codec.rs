//! Primitive protocol codecs: VarInt, strings and packet framing.

use std::fmt;
use std::io::{self, Read};

/// Maximum number of bytes a VarInt can occupy.
pub const MAX_VARINT_BYTES: usize = 5;

/// Errors produced while decoding protocol data.
#[derive(Debug)]
pub enum CodecError {
    /// The underlying reader failed (including unexpected end of stream).
    Io(io::Error),
    /// A VarInt used more than [`MAX_VARINT_BYTES`] bytes.
    VarIntTooLong,
    /// A length prefix was negative.
    NegativeLength(i32),
    /// A length prefix exceeded the allowed maximum.
    TooLong {
        /// The declared length.
        len: usize,
        /// The maximum accepted length.
        max: usize,
    },
    /// A string was not valid UTF-8.
    InvalidUtf8,
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::VarIntTooLong => write!(f, "VarInt is longer than 5 bytes"),
            Self::NegativeLength(n) => write!(f, "negative length prefix {n}"),
            Self::TooLong { len, max } => write!(f, "length {len} exceeds maximum {max}"),
            Self::InvalidUtf8 => write!(f, "string is not valid UTF-8"),
        }
    }
}

impl std::error::Error for CodecError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for CodecError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

/// Append `value` to `out` as a VarInt.
pub fn write_varint(out: &mut Vec<u8>, value: i32) {
    let mut v = value as u32;
    loop {
        let byte = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

/// Read a VarInt from `reader`.
pub fn read_varint<R: Read>(reader: &mut R) -> Result<i32, CodecError> {
    let mut result: u32 = 0;
    for i in 0..MAX_VARINT_BYTES {
        let mut byte = [0u8; 1];
        reader.read_exact(&mut byte)?;
        result |= u32::from(byte[0] & 0x7f) << (7 * i);
        if byte[0] & 0x80 == 0 {
            return Ok(result as i32);
        }
    }
    Err(CodecError::VarIntTooLong)
}

/// Append a length-prefixed UTF-8 string to `out`.
pub fn write_string(out: &mut Vec<u8>, s: &str) {
    write_varint(out, s.len() as i32);
    out.extend_from_slice(s.as_bytes());
}

/// Read a length-prefixed UTF-8 string of at most `max_bytes` bytes.
pub fn read_string<R: Read>(reader: &mut R, max_bytes: usize) -> Result<String, CodecError> {
    let len = read_varint(reader)?;
    if len < 0 {
        return Err(CodecError::NegativeLength(len));
    }
    let len = len as usize;
    if len > max_bytes {
        return Err(CodecError::TooLong {
            len,
            max: max_bytes,
        });
    }
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    String::from_utf8(buf).map_err(|_| CodecError::InvalidUtf8)
}

/// Frame a packet: `VarInt(length) | VarInt(id) | body`.
pub fn frame_packet(id: i32, body: &[u8]) -> Vec<u8> {
    let mut inner = Vec::with_capacity(body.len() + MAX_VARINT_BYTES);
    write_varint(&mut inner, id);
    inner.extend_from_slice(body);
    let mut out = Vec::with_capacity(inner.len() + MAX_VARINT_BYTES);
    write_varint(&mut out, inner.len() as i32);
    out.extend_from_slice(&inner);
    out
}

/// Read one uncompressed packet and return its id and body.
///
/// `max_len` bounds the declared packet length so a hostile peer cannot force a huge allocation.
pub fn read_packet<R: Read>(reader: &mut R, max_len: usize) -> Result<(i32, Vec<u8>), CodecError> {
    let len = read_varint(reader)?;
    if len <= 0 {
        return Err(CodecError::NegativeLength(len));
    }
    let len = len as usize;
    if len > max_len {
        return Err(CodecError::TooLong { len, max: max_len });
    }
    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload)?;
    let mut cursor = io::Cursor::new(&payload);
    let id = read_varint(&mut cursor)?;
    let offset = cursor.position() as usize;
    Ok((id, payload[offset..].to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(value: i32) -> i32 {
        let mut buf = Vec::new();
        write_varint(&mut buf, value);
        read_varint(&mut buf.as_slice()).unwrap()
    }

    #[test]
    fn varint_roundtrips_edge_values() {
        for v in [
            0,
            1,
            127,
            128,
            255,
            25565,
            2_097_151,
            i32::MAX,
            -1,
            i32::MIN,
        ] {
            assert_eq!(roundtrip(v), v);
        }
    }

    #[test]
    fn varint_known_encodings() {
        let mut buf = Vec::new();
        write_varint(&mut buf, 300);
        assert_eq!(buf, [0xac, 0x02]);
        buf.clear();
        write_varint(&mut buf, -1);
        assert_eq!(buf, [0xff, 0xff, 0xff, 0xff, 0x0f]);
    }

    #[test]
    fn varint_too_long_is_rejected() {
        let bytes = [0x80u8; 6];
        assert!(matches!(
            read_varint(&mut bytes.as_slice()),
            Err(CodecError::VarIntTooLong)
        ));
    }

    #[test]
    fn varint_truncated_is_io_error() {
        let bytes = [0x80u8];
        assert!(matches!(
            read_varint(&mut bytes.as_slice()),
            Err(CodecError::Io(_))
        ));
    }

    #[test]
    fn string_roundtrip_and_limits() {
        let mut buf = Vec::new();
        write_string(&mut buf, "héllo");
        assert_eq!(read_string(&mut buf.as_slice(), 64).unwrap(), "héllo");
        assert!(matches!(
            read_string(&mut buf.as_slice(), 2),
            Err(CodecError::TooLong { .. })
        ));
    }

    #[test]
    fn invalid_utf8_is_rejected() {
        let buf = [2u8, 0xff, 0xfe];
        assert!(matches!(
            read_string(&mut buf.as_slice(), 8),
            Err(CodecError::InvalidUtf8)
        ));
    }

    #[test]
    fn packet_frame_roundtrip() {
        let framed = frame_packet(0x01, &[1, 2, 3]);
        let (id, body) = read_packet(&mut framed.as_slice(), 64).unwrap();
        assert_eq!(id, 1);
        assert_eq!(body, [1, 2, 3]);
    }

    #[test]
    fn oversized_packet_is_rejected() {
        let framed = frame_packet(0, &[0u8; 100]);
        assert!(matches!(
            read_packet(&mut framed.as_slice(), 10),
            Err(CodecError::TooLong { .. })
        ));
    }
}
