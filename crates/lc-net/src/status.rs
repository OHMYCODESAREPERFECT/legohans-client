//! Blocking server-list status query (handshake, status request, ping).

use std::fmt;
use std::io::{self, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use lc_protocol::PROTOCOL_VERSION;
use lc_protocol::codec::{CodecError, read_packet, read_string};
use lc_protocol::status::{
    MAX_STATUS_JSON_BYTES, MAX_STATUS_PACKET_BYTES, PING_PACKET_ID, STATUS_PACKET_ID,
    handshake_packet, ping_packet, status_request_packet,
};

/// A successful status query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusReply {
    /// The raw status JSON returned by the server.
    pub json: String,
    /// Round-trip time of the ping/pong exchange, or of the status request if the
    /// server did not answer the ping.
    pub latency: Duration,
}

/// Why a status query failed.
#[derive(Debug)]
pub enum StatusError {
    /// The host name could not be resolved to any address.
    Resolve(String),
    /// Connecting or talking to the server failed.
    Io(io::Error),
    /// The server sent data that does not follow the protocol.
    Protocol(CodecError),
    /// The server answered with an unexpected packet id.
    UnexpectedPacket(i32),
}

impl fmt::Display for StatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Resolve(host) => write!(f, "could not resolve {host}"),
            Self::Io(e) => write!(f, "{e}"),
            Self::Protocol(e) => write!(f, "invalid response: {e}"),
            Self::UnexpectedPacket(id) => write!(f, "unexpected packet id {id}"),
        }
    }
}

impl std::error::Error for StatusError {}

impl From<io::Error> for StatusError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<CodecError> for StatusError {
    fn from(e: CodecError) -> Self {
        Self::Protocol(e)
    }
}

/// Query the status of the server at `host:port`.
///
/// `timeout` applies to connecting and to every read and write. SRV record lookup is not
/// performed: the address is used as given.
pub fn query_status(host: &str, port: u16, timeout: Duration) -> Result<StatusReply, StatusError> {
    let addrs: Vec<_> = (host, port)
        .to_socket_addrs()
        .map_err(|_| StatusError::Resolve(host.to_string()))?
        .collect();
    if addrs.is_empty() {
        return Err(StatusError::Resolve(host.to_string()));
    }

    let mut last_error = None;
    let mut stream = None;
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(s) => {
                stream = Some(s);
                break;
            }
            Err(e) => last_error = Some(e),
        }
    }
    let mut stream = match stream {
        Some(s) => s,
        None => {
            return Err(StatusError::Io(last_error.unwrap_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "no address to connect to")
            })));
        }
    };
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    stream.set_nodelay(true)?;

    let started = Instant::now();
    stream.write_all(&handshake_packet(PROTOCOL_VERSION, host, port))?;
    stream.write_all(&status_request_packet())?;
    stream.flush()?;

    let (id, body) = read_packet(&mut stream, MAX_STATUS_PACKET_BYTES)?;
    if id != STATUS_PACKET_ID {
        return Err(StatusError::UnexpectedPacket(id));
    }
    let json = read_string(&mut body.as_slice(), MAX_STATUS_JSON_BYTES)?;
    let status_rtt = started.elapsed();

    // The ping is best-effort: some servers close right after the status response.
    let ping_started = Instant::now();
    let latency = match stream
        .write_all(&ping_packet(0x4c43))
        .and_then(|()| stream.flush())
    {
        Ok(()) => match read_packet(&mut stream, 64) {
            Ok((PING_PACKET_ID, _)) => ping_started.elapsed(),
            _ => status_rtt,
        },
        Err(_) => status_rtt,
    };

    Ok(StatusReply { json, latency })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lc_protocol::codec::{frame_packet, read_varint, write_string};
    use std::io::Read;
    use std::net::TcpListener;
    use std::thread;

    /// Spawn a one-shot mock server. `answer_ping` controls whether it echoes the ping.
    fn mock_server(json: &'static str, answer_ping: bool) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let (hs_id, _) = read_packet(&mut stream, 1024).unwrap();
            assert_eq!(hs_id, 0);
            let (req_id, _) = read_packet(&mut stream, 1024).unwrap();
            assert_eq!(req_id, 0);
            let mut body = Vec::new();
            write_string(&mut body, json);
            stream.write_all(&frame_packet(0, &body)).unwrap();
            if answer_ping {
                let (ping_id, payload) = read_packet(&mut stream, 1024).unwrap();
                assert_eq!(ping_id, 1);
                stream.write_all(&frame_packet(1, &payload)).unwrap();
            }
            let mut sink = [0u8; 1];
            let _ = stream.read(&mut sink);
        });
        port
    }

    #[test]
    fn queries_a_mock_server() {
        let port = mock_server(r#"{"players":{"online":3,"max":20}}"#, true);
        let reply = query_status("127.0.0.1", port, Duration::from_secs(3)).unwrap();
        assert!(reply.json.contains("\"online\":3"));
        assert!(reply.latency < Duration::from_secs(3));
    }

    #[test]
    fn works_when_server_ignores_ping() {
        let port = mock_server("{}", false);
        let reply = query_status("127.0.0.1", port, Duration::from_millis(500)).unwrap();
        assert_eq!(reply.json, "{}");
    }

    #[test]
    fn closed_port_is_an_error() {
        let port = {
            let l = TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap().port()
        };
        assert!(query_status("127.0.0.1", port, Duration::from_millis(500)).is_err());
    }

    #[test]
    fn wrong_packet_id_is_reported() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = read_varint(&mut stream);
            thread::sleep(Duration::from_millis(50));
            let _ = stream.write_all(&frame_packet(0x7f, &[0]));
        });
        let err = query_status("127.0.0.1", port, Duration::from_secs(2)).unwrap_err();
        assert!(matches!(
            err,
            StatusError::UnexpectedPacket(0x7f) | StatusError::Io(_) | StatusError::Protocol(_)
        ));
    }
}
