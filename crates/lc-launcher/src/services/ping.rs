//! Server status pinging on background threads.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use serde_json::Value;

use crate::model::server::ServerAddress;
use crate::util::{base64_decode, component_text};

const PING_TIMEOUT: Duration = Duration::from_secs(4);
const MAX_CONCURRENT: usize = 6;

/// Interesting fields of a status response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedStatus {
    /// Version name reported by the server.
    pub version_name: String,
    /// Players online, if reported.
    pub players_online: Option<u32>,
    /// Player capacity, if reported.
    pub players_max: Option<u32>,
    /// Message of the day as plain text.
    pub motd: String,
    /// Server icon PNG bytes, if provided.
    pub favicon_png: Option<Vec<u8>>,
}

/// Parse the status JSON returned by a server.
pub fn parse_status_json(json: &str) -> Result<ParsedStatus, String> {
    let root: Value =
        serde_json::from_str(json).map_err(|e| format!("invalid status JSON: {e}"))?;
    if !root.is_object() {
        return Err("status response is not a JSON object".to_string());
    }
    let count = |path: &str| {
        root.get("players")
            .and_then(|p| p.get(path))
            .and_then(Value::as_u64)
            .map(|n| n.min(u64::from(u32::MAX)) as u32)
    };
    let favicon_png = root
        .get("favicon")
        .and_then(Value::as_str)
        .and_then(|s| s.strip_prefix("data:image/png;base64,"))
        .and_then(base64_decode);
    Ok(ParsedStatus {
        version_name: root
            .get("version")
            .and_then(|v| v.get("name"))
            .and_then(Value::as_str)
            .map(crate::util::strip_formatting)
            .unwrap_or_default(),
        players_online: count("online"),
        players_max: count("max"),
        motd: root
            .get("description")
            .map(component_text)
            .unwrap_or_default()
            .trim()
            .to_string(),
        favicon_png,
    })
}

/// What is known about a server right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerState {
    /// A ping has been queued or is running.
    Pinging,
    /// The server answered.
    Online {
        /// Round-trip time in milliseconds.
        latency_ms: u64,
        /// The parsed response.
        status: Box<ParsedStatus>,
    },
    /// The ping failed, with the reason.
    Offline(String),
}

type PingResult = Result<(ParsedStatus, u64), String>;

/// Runs status pings on worker threads and collects results.
pub struct PingManager {
    tx: Sender<(String, PingResult)>,
    rx: Receiver<(String, PingResult)>,
    in_flight: HashSet<String>,
    queue: VecDeque<(String, ServerAddress)>,
    /// Latest known state per server id.
    pub states: HashMap<String, ServerState>,
    repaint: Option<egui::Context>,
}

impl PingManager {
    /// Create a manager. If `repaint` is given the UI is woken when a result arrives.
    pub fn new(repaint: Option<egui::Context>) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx,
            in_flight: HashSet::new(),
            queue: VecDeque::new(),
            states: HashMap::new(),
            repaint,
        }
    }

    /// Whether any ping is still running or queued.
    pub fn is_busy(&self) -> bool {
        !self.in_flight.is_empty() || !self.queue.is_empty()
    }

    /// Request a ping of the server `id`. Ignored if one is already pending for it.
    pub fn request(&mut self, id: &str, address: ServerAddress) {
        if self.in_flight.contains(id) || self.queue.iter().any(|(q, _)| q == id) {
            return;
        }
        self.states.insert(id.to_string(), ServerState::Pinging);
        self.queue.push_back((id.to_string(), address));
        self.start_queued();
    }

    /// Forget a removed server.
    pub fn forget(&mut self, id: &str) {
        self.states.remove(id);
        self.queue.retain(|(q, _)| q != id);
    }

    fn start_queued(&mut self) {
        while self.in_flight.len() < MAX_CONCURRENT {
            let Some((id, address)) = self.queue.pop_front() else {
                break;
            };
            self.in_flight.insert(id.clone());
            let tx = self.tx.clone();
            let repaint = self.repaint.clone();
            let spawned = thread::Builder::new()
                .name("lc-ping".into())
                .spawn(move || {
                    let result =
                        lc_net::status::query_status(&address.host, address.port, PING_TIMEOUT)
                            .map_err(|e| e.to_string())
                            .and_then(|reply| {
                                parse_status_json(&reply.json)
                                    .map(|status| (status, reply.latency.as_millis() as u64))
                            });
                    let _ = tx.send((id, result));
                    if let Some(ctx) = repaint {
                        ctx.request_repaint();
                    }
                });
            if spawned.is_err() {
                // Could not start a thread; report it instead of hanging in "pinging".
                // The id was moved into the closure, so recover it from the pending set.
                let stuck: Vec<String> = self.in_flight.drain().collect();
                for id in stuck {
                    self.states.insert(
                        id,
                        ServerState::Offline("could not start a ping thread".into()),
                    );
                }
                break;
            }
        }
    }

    /// Collect finished pings. Returns whether anything changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok((id, result)) = self.rx.try_recv() {
            self.in_flight.remove(&id);
            // A result for a server removed in the meantime is dropped.
            if let Some(slot) = self.states.get_mut(&id) {
                let state = match result {
                    Ok((status, latency_ms)) => ServerState::Online {
                        latency_ms,
                        status: Box::new(status),
                    },
                    Err(e) => ServerState::Offline(e),
                };
                *slot = state;
                changed = true;
            }
        }
        self.start_queued();
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lc_protocol::codec::{frame_packet, read_packet, write_string};
    use std::io::Write;
    use std::net::TcpListener;
    use std::time::Instant;

    #[test]
    fn parses_a_typical_response() {
        let json = r#"{
            "version": {"name": "Paper 1.21.11", "protocol": 774},
            "players": {"max": 100, "online": 5, "sample": []},
            "description": {"text": "Welcome ", "extra": [{"text": "§aHome"}]},
            "favicon": "data:image/png;base64,aGVsbG8="
        }"#;
        let s = parse_status_json(json).unwrap();
        assert_eq!(s.version_name, "Paper 1.21.11");
        assert_eq!((s.players_online, s.players_max), (Some(5), Some(100)));
        assert_eq!(s.motd, "Welcome Home");
        assert_eq!(s.favicon_png.as_deref(), Some(&b"hello"[..]));
    }

    #[test]
    fn tolerates_missing_fields_and_plain_string_motd() {
        let s = parse_status_json(r#"{"description":"Plain"}"#).unwrap();
        assert_eq!(s.motd, "Plain");
        assert_eq!(s.players_online, None);
        assert!(s.favicon_png.is_none());
    }

    #[test]
    fn rejects_non_objects_and_garbage() {
        assert!(parse_status_json("[]").is_err());
        assert!(parse_status_json("nope").is_err());
    }

    fn spawn_mock(json: &'static str) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let _ = read_packet(&mut stream, 1024);
                let _ = read_packet(&mut stream, 1024);
                let mut body = Vec::new();
                write_string(&mut body, json);
                let _ = stream.write_all(&frame_packet(0, &body));
                if let Ok((_, payload)) = read_packet(&mut stream, 1024) {
                    let _ = stream.write_all(&frame_packet(1, &payload));
                }
            }
        });
        port
    }

    fn wait_for(manager: &mut PingManager, id: &str) -> ServerState {
        let deadline = Instant::now() + Duration::from_secs(8);
        while Instant::now() < deadline {
            manager.poll();
            if let Some(state) = manager.states.get(id)
                && *state != ServerState::Pinging
            {
                return state.clone();
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("ping did not finish");
    }

    #[test]
    fn manager_reports_online_server() {
        let port = spawn_mock(r#"{"players":{"online":2,"max":10},"description":"Hi"}"#);
        let mut m = PingManager::new(None);
        m.request(
            "s1",
            ServerAddress {
                host: "127.0.0.1".into(),
                port,
            },
        );
        assert_eq!(m.states["s1"], ServerState::Pinging);
        assert!(m.is_busy());
        match wait_for(&mut m, "s1") {
            ServerState::Online { status, .. } => {
                assert_eq!(status.players_online, Some(2));
                assert_eq!(status.motd, "Hi");
            }
            other => panic!("expected online, got {other:?}"),
        }
        assert!(!m.is_busy());
    }

    #[test]
    fn manager_reports_offline_server() {
        let port = {
            let l = TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap().port()
        };
        let mut m = PingManager::new(None);
        m.request(
            "s2",
            ServerAddress {
                host: "127.0.0.1".into(),
                port,
            },
        );
        assert!(matches!(wait_for(&mut m, "s2"), ServerState::Offline(_)));
    }

    #[test]
    fn duplicate_requests_and_forgetting() {
        let mut m = PingManager::new(None);
        let addr = ServerAddress {
            host: "127.0.0.1".into(),
            port: 1,
        };
        m.request("a", addr.clone());
        m.request("a", addr);
        assert_eq!(m.in_flight.len() + m.queue.len(), 1);
        m.forget("a");
        assert!(!m.states.contains_key("a"));
    }
}
