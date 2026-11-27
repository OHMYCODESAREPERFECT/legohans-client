//! Saved servers.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::util::{new_id, now_secs};

/// Default Minecraft server port.
pub const DEFAULT_PORT: u16 = 25565;

/// Why a server entry is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerError {
    /// The name is empty.
    EmptyName,
    /// The name is longer than 48 characters.
    NameTooLong,
    /// The address is empty.
    EmptyAddress,
    /// The address is malformed.
    BadAddress,
    /// The port is not a number in 1..=65535.
    BadPort,
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => write!(f, "Give the server a name."),
            Self::NameTooLong => write!(f, "The name can be at most 48 characters."),
            Self::EmptyAddress => write!(f, "Enter the server address."),
            Self::BadAddress => write!(f, "That address does not look valid."),
            Self::BadPort => write!(f, "The port must be a number from 1 to 65535."),
        }
    }
}

/// A parsed `host[:port]` address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerAddress {
    /// Host name or IP address (without brackets).
    pub host: String,
    /// TCP port.
    pub port: u16,
}

impl ServerAddress {
    /// Parse `host`, `host:port`, `1.2.3.4:port` or `[::1]:port`.
    pub fn parse(input: &str) -> Result<Self, ServerError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ServerError::EmptyAddress);
        }
        let (host, port_text) = if let Some(rest) = input.strip_prefix('[') {
            let (host, tail) = rest.split_once(']').ok_or(ServerError::BadAddress)?;
            match tail {
                "" => (host, None),
                _ => (
                    host,
                    Some(tail.strip_prefix(':').ok_or(ServerError::BadAddress)?),
                ),
            }
        } else {
            match input.matches(':').count() {
                0 => (input, None),
                1 => {
                    let (h, p) = input.split_once(':').ok_or(ServerError::BadAddress)?;
                    (h, Some(p))
                }
                _ => return Err(ServerError::BadAddress),
            }
        };
        let valid_host = !host.is_empty()
            && host.len() <= 253
            && host
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':'));
        if !valid_host {
            return Err(ServerError::BadAddress);
        }
        let port = match port_text {
            None => DEFAULT_PORT,
            Some(p) => match p.parse::<u16>() {
                Ok(n) if n >= 1 => n,
                _ => return Err(ServerError::BadPort),
            },
        };
        Ok(Self {
            host: host.to_string(),
            port,
        })
    }
}

impl fmt::Display for ServerAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let host = if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        if self.port == DEFAULT_PORT {
            write!(f, "{host}")
        } else {
            write!(f, "{host}:{}", self.port)
        }
    }
}

/// A saved server.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerEntry {
    /// Unique id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// `host[:port]`.
    pub address: String,
    /// Your own description or notes.
    pub description: String,
    /// Pinned as favorite.
    pub favorite: bool,
    /// Last time the game was launched into this server (Unix seconds).
    pub last_played: Option<u64>,
}

impl ServerEntry {
    /// Create an entry with a fresh id.
    pub fn new(name: &str, address: &str, description: &str) -> Self {
        Self {
            id: new_id("server"),
            name: name.trim().to_string(),
            address: address.trim().to_string(),
            description: description.trim().to_string(),
            ..Self::default()
        }
    }

    /// Validate the entry and return its parsed address.
    pub fn validate(&self) -> Result<ServerAddress, ServerError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(ServerError::EmptyName);
        }
        if name.chars().count() > 48 {
            return Err(ServerError::NameTooLong);
        }
        ServerAddress::parse(&self.address)
    }
}

/// The persisted list of servers.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerStore {
    /// All servers in display order.
    pub servers: Vec<ServerEntry>,
}

impl ServerStore {
    /// Find a server by id.
    pub fn find(&self, id: &str) -> Option<&ServerEntry> {
        self.servers.iter().find(|s| s.id == id)
    }

    /// Add a validated server; returns its id.
    pub fn add(&mut self, entry: ServerEntry) -> Result<String, ServerError> {
        entry.validate()?;
        let id = entry.id.clone();
        self.servers.push(entry);
        Ok(id)
    }

    /// Replace the server with the same id by a validated one.
    pub fn update(&mut self, entry: ServerEntry) -> Result<(), ServerError> {
        entry.validate()?;
        if let Some(slot) = self.servers.iter_mut().find(|s| s.id == entry.id) {
            *slot = entry;
        }
        Ok(())
    }

    /// Remove a server; returns whether it existed.
    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.servers.len();
        self.servers.retain(|s| s.id != id);
        self.servers.len() != before
    }

    /// Flip the favorite flag.
    pub fn toggle_favorite(&mut self, id: &str) {
        if let Some(s) = self.servers.iter_mut().find(|s| s.id == id) {
            s.favorite = !s.favorite;
        }
    }

    /// Record that the game was launched into this server now.
    pub fn mark_played(&mut self, id: &str) {
        if let Some(s) = self.servers.iter_mut().find(|s| s.id == id) {
            s.last_played = Some(now_secs());
        }
    }

    /// Servers that have been played, most recent first.
    pub fn recent(&self, limit: usize) -> Vec<&ServerEntry> {
        let mut played: Vec<&ServerEntry> = self
            .servers
            .iter()
            .filter(|s| s.last_played.is_some())
            .collect();
        played.sort_by_key(|s| std::cmp::Reverse(s.last_played));
        played.truncate(limit);
        played
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_addresses() {
        let a = ServerAddress::parse("play.example.org").unwrap();
        assert_eq!((a.host.as_str(), a.port), ("play.example.org", 25565));
        let a = ServerAddress::parse(" 10.0.0.1:25570 ").unwrap();
        assert_eq!((a.host.as_str(), a.port), ("10.0.0.1", 25570));
        let a = ServerAddress::parse("[::1]:25566").unwrap();
        assert_eq!((a.host.as_str(), a.port), ("::1", 25566));
        let a = ServerAddress::parse("[::1]").unwrap();
        assert_eq!(a.port, 25565);
    }

    #[test]
    fn rejects_bad_addresses() {
        assert_eq!(ServerAddress::parse(""), Err(ServerError::EmptyAddress));
        assert_eq!(ServerAddress::parse("a b"), Err(ServerError::BadAddress));
        assert_eq!(ServerAddress::parse("::1"), Err(ServerError::BadAddress));
        assert_eq!(ServerAddress::parse("[::1"), Err(ServerError::BadAddress));
        assert_eq!(ServerAddress::parse("host:0"), Err(ServerError::BadPort));
        assert_eq!(
            ServerAddress::parse("host:99999"),
            Err(ServerError::BadPort)
        );
        assert_eq!(ServerAddress::parse("host:abc"), Err(ServerError::BadPort));
        assert_eq!(ServerAddress::parse(":25565"), Err(ServerError::BadAddress));
    }

    #[test]
    fn display_omits_default_port_and_brackets_ipv6() {
        assert_eq!(ServerAddress::parse("a.b").unwrap().to_string(), "a.b");
        assert_eq!(
            ServerAddress::parse("a.b:1234").unwrap().to_string(),
            "a.b:1234"
        );
        assert_eq!(
            ServerAddress::parse("[::1]:1234").unwrap().to_string(),
            "[::1]:1234"
        );
    }

    #[test]
    fn entry_validation() {
        assert_eq!(
            ServerEntry::new("", "a.b", "").validate(),
            Err(ServerError::EmptyName)
        );
        assert_eq!(
            ServerEntry::new(&"x".repeat(49), "a.b", "").validate(),
            Err(ServerError::NameTooLong)
        );
        assert!(ServerEntry::new("ok", "a.b", "").validate().is_ok());
    }

    #[test]
    fn store_operations() {
        let mut s = ServerStore::default();
        let a = s.add(ServerEntry::new("A", "a.example", "first")).unwrap();
        let b = s.add(ServerEntry::new("B", "b.example", "")).unwrap();
        assert!(s.add(ServerEntry::new("C", "bad address", "")).is_err());
        assert_eq!(s.servers.len(), 2);

        s.toggle_favorite(&a);
        assert!(s.find(&a).unwrap().favorite);
        s.toggle_favorite(&a);
        assert!(!s.find(&a).unwrap().favorite);

        let mut edited = s.find(&b).unwrap().clone();
        edited.name = "B2".into();
        s.update(edited).unwrap();
        assert_eq!(s.find(&b).unwrap().name, "B2");

        assert!(s.remove(&a));
        assert!(!s.remove(&a));
    }

    #[test]
    fn recent_is_sorted_and_limited() {
        let mut s = ServerStore::default();
        let a = s.add(ServerEntry::new("A", "a.example", "")).unwrap();
        let b = s.add(ServerEntry::new("B", "b.example", "")).unwrap();
        s.add(ServerEntry::new("C", "c.example", "")).unwrap();
        assert!(s.recent(5).is_empty());
        s.find_mut_for_test(&a).last_played = Some(100);
        s.find_mut_for_test(&b).last_played = Some(200);
        let recent = s.recent(5);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].id, b);
        assert_eq!(s.recent(1).len(), 1);
        s.mark_played(&a);
        assert_eq!(s.recent(5)[0].id, a, "mark_played records the current time");
    }

    impl ServerStore {
        fn find_mut_for_test(&mut self, id: &str) -> &mut ServerEntry {
            self.servers.iter_mut().find(|s| s.id == id).unwrap()
        }
    }
}
