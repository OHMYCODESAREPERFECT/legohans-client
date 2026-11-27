//! Launcher accounts: offline profiles and (not yet usable) Microsoft accounts.

use std::fmt;

use lc_net::AccountKind;
use serde::{Deserialize, Serialize};

use crate::util::{new_id, now_secs};

/// Why an offline username was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountError {
    /// Shorter than 3 or longer than 16 characters.
    BadLength,
    /// Contains characters other than letters, digits and underscore.
    BadCharacters,
    /// An account with that name already exists.
    Duplicate,
}

impl fmt::Display for AccountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadLength => write!(f, "Usernames must be 3 to 16 characters long."),
            Self::BadCharacters => write!(f, "Use only letters, digits and underscores."),
            Self::Duplicate => write!(f, "An account with that name already exists."),
        }
    }
}

/// Validate a Minecraft username and return it trimmed.
pub fn validate_username(name: &str) -> Result<String, AccountError> {
    let name = name.trim();
    let len = name.chars().count();
    if !(3..=16).contains(&len) {
        return Err(AccountError::BadLength);
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(AccountError::BadCharacters);
    }
    Ok(name.to_string())
}

/// A stored account.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AccountRecord {
    /// A local profile for servers that allow offline mode.
    Offline {
        /// Unique id.
        id: String,
        /// In-game name.
        username: String,
        /// Creation time (Unix seconds).
        added: u64,
    },
    /// A Microsoft account. Sign-in is not implemented yet, so no tokens are ever stored
    /// and such an account can never launch the game.
    Microsoft {
        /// Unique id.
        id: String,
        /// Profile name.
        username: String,
        /// Creation time (Unix seconds).
        added: u64,
    },
}

impl AccountRecord {
    /// The unique id.
    pub fn id(&self) -> &str {
        match self {
            Self::Offline { id, .. } | Self::Microsoft { id, .. } => id,
        }
    }

    /// The in-game name.
    pub fn username(&self) -> &str {
        match self {
            Self::Offline { username, .. } | Self::Microsoft { username, .. } => username,
        }
    }

    /// Creation time.
    pub fn added(&self) -> u64 {
        match self {
            Self::Offline { added, .. } | Self::Microsoft { added, .. } => *added,
        }
    }

    /// Whether this account can start a game session today.
    pub fn can_launch(&self) -> bool {
        matches!(self, Self::Offline { .. })
    }

    /// Short description of the account type and its state.
    pub fn status_label(&self) -> &'static str {
        match self {
            Self::Offline { .. } => "Offline profile",
            Self::Microsoft { .. } => "Microsoft: sign-in not available yet",
        }
    }

    /// Convert to the networking layer's account kind.
    pub fn net_kind(&self) -> AccountKind {
        match self {
            Self::Offline { username, .. } => AccountKind::Offline {
                username: username.clone(),
            },
            Self::Microsoft { .. } => AccountKind::Microsoft,
        }
    }
}

/// The persisted list of accounts.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AccountStore {
    /// All accounts in display order.
    pub accounts: Vec<AccountRecord>,
}

impl AccountStore {
    /// Find an account by id.
    pub fn find(&self, id: &str) -> Option<&AccountRecord> {
        self.accounts.iter().find(|a| a.id() == id)
    }

    /// Add an offline profile; returns its id.
    pub fn add_offline(&mut self, username: &str) -> Result<String, AccountError> {
        let username = validate_username(username)?;
        if self
            .accounts
            .iter()
            .any(|a| a.username().eq_ignore_ascii_case(&username))
        {
            return Err(AccountError::Duplicate);
        }
        let id = new_id("account");
        self.accounts.push(AccountRecord::Offline {
            id: id.clone(),
            username,
            added: now_secs(),
        });
        Ok(id)
    }

    /// Remove an account; returns whether it existed.
    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.accounts.len();
        self.accounts.retain(|a| a.id() != id);
        self.accounts.len() != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usernames_are_validated() {
        assert_eq!(validate_username("  Steve "), Ok("Steve".to_string()));
        assert_eq!(validate_username("ab"), Err(AccountError::BadLength));
        assert_eq!(
            validate_username("a_very_long_name_indeed"),
            Err(AccountError::BadLength)
        );
        assert_eq!(
            validate_username("bad name"),
            Err(AccountError::BadCharacters)
        );
        assert_eq!(validate_username("héllo"), Err(AccountError::BadCharacters));
        assert!(validate_username("Player_1").is_ok());
    }

    #[test]
    fn duplicates_are_case_insensitive() {
        let mut s = AccountStore::default();
        s.add_offline("Steve").unwrap();
        assert_eq!(s.add_offline("steve"), Err(AccountError::Duplicate));
        assert_eq!(s.accounts.len(), 1);
    }

    #[test]
    fn add_find_remove() {
        let mut s = AccountStore::default();
        let id = s.add_offline("Alex").unwrap();
        assert_eq!(s.find(&id).unwrap().username(), "Alex");
        assert!(s.remove(&id));
        assert!(!s.remove(&id));
    }

    #[test]
    fn offline_can_launch_microsoft_cannot() {
        let off = AccountRecord::Offline {
            id: "a".into(),
            username: "Alex".into(),
            added: 0,
        };
        let ms = AccountRecord::Microsoft {
            id: "b".into(),
            username: "Alex".into(),
            added: 0,
        };
        assert!(off.can_launch() && !ms.can_launch());
        assert!(!off.net_kind().requires_online_auth());
        assert!(ms.net_kind().requires_online_auth());
    }

    #[test]
    fn serde_uses_a_type_tag() {
        let mut s = AccountStore::default();
        s.add_offline("Alex").unwrap();
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"type\":\"Offline\""));
        let back: AccountStore = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }
}
