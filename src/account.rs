use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Access level hierarchy for server accounts.
/// Ordering: Player < Counselor < GameMaster < Seer < Administrator < Developer < Owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessLevel {
    Player,
    Counselor,
    GameMaster,
    Seer,
    Administrator,
    Developer,
    Owner,
}

impl AccessLevel {
    fn rank(self) -> u8 {
        match self {
            AccessLevel::Player => 0,
            AccessLevel::Counselor => 1,
            AccessLevel::GameMaster => 2,
            AccessLevel::Seer => 3,
            AccessLevel::Administrator => 4,
            AccessLevel::Developer => 5,
            AccessLevel::Owner => 6,
        }
    }
}

impl PartialOrd for AccessLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.rank().cmp(&other.rank()))
    }
}

impl Ord for AccessLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.rank().cmp(&other.rank())
    }
}

/// A player account on the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub username: String,
    pub password_hash: String,
    pub access_level: AccessLevel,
    pub character_slots: [Option<String>; 7],
    pub created_at: i64,
    pub last_login: i64,
    pub banned: bool,
    pub ban_reason: Option<String>,
}

/// Simple hash-based password "hashing" (not cryptographic — suitable for a game server prototype).
fn hash_password(password: &str) -> String {
    let mut hasher = DefaultHasher::new();
    // Salt with a fixed prefix so raw passwords aren't directly recoverable via rainbow table.
    "uo-server-salt:".hash(&mut hasher);
    password.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

impl Account {
    /// Create a new account with the given username and plaintext password.
    pub fn new(username: &str, password: &str) -> Self {
        let now = chrono::Utc::now().timestamp();
        Account {
            username: username.to_string(),
            password_hash: hash_password(password),
            access_level: AccessLevel::Player,
            character_slots: Default::default(),
            created_at: now,
            last_login: now,
            banned: false,
            ban_reason: None,
        }
    }

    /// Check whether the given plaintext password matches the stored hash.
    pub fn verify_password(&self, password: &str) -> bool {
        hash_password(password) == self.password_hash
    }

    /// Update the password (takes plaintext, stores hash).
    pub fn set_password(&mut self, password: &str) {
        self.password_hash = hash_password(password);
    }

    /// Returns `true` if this account's access level is >= the required level.
    pub fn has_access(&self, required: AccessLevel) -> bool {
        self.access_level >= required
    }

    /// Place a character name into the first available slot.
    /// Returns `Ok(slot_index)` on success or `Err` if all slots are full.
    pub fn add_character(&mut self, name: String) -> Result<usize, String> {
        for (i, slot) in self.character_slots.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(name);
                return Ok(i);
            }
        }
        Err("All character slots are full".to_string())
    }

    /// Remove a character by name. Returns `true` if found and removed.
    pub fn remove_character(&mut self, name: &str) -> bool {
        for slot in self.character_slots.iter_mut() {
            if slot.as_deref() == Some(name) {
                *slot = None;
                return true;
            }
        }
        false
    }

    /// Ban this account with the given reason.
    pub fn ban(&mut self, reason: &str) {
        self.banned = true;
        self.ban_reason = Some(reason.to_string());
    }

    /// Unban this account.
    pub fn unban(&mut self) {
        self.banned = false;
        self.ban_reason = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_account_defaults() {
        let acc = Account::new("testuser", "pass123");
        assert_eq!(acc.username, "testuser");
        assert_eq!(acc.access_level, AccessLevel::Player);
        assert!(!acc.banned);
        assert!(acc.ban_reason.is_none());
        assert!(acc.character_slots.iter().all(|s| s.is_none()));
    }

    #[test]
    fn test_verify_password() {
        let acc = Account::new("user", "secret");
        assert!(acc.verify_password("secret"));
        assert!(!acc.verify_password("wrong"));
    }

    #[test]
    fn test_set_password() {
        let mut acc = Account::new("user", "old");
        assert!(acc.verify_password("old"));
        acc.set_password("new");
        assert!(!acc.verify_password("old"));
        assert!(acc.verify_password("new"));
    }

    #[test]
    fn test_access_level_ordering() {
        assert!(AccessLevel::Owner > AccessLevel::Player);
        assert!(AccessLevel::GameMaster > AccessLevel::Counselor);
        assert!(AccessLevel::Administrator > AccessLevel::Seer);
        assert!(AccessLevel::Developer > AccessLevel::Administrator);
        assert!(AccessLevel::Player < AccessLevel::Owner);
    }

    #[test]
    fn test_has_access() {
        let mut acc = Account::new("admin", "pass");
        acc.access_level = AccessLevel::GameMaster;
        assert!(acc.has_access(AccessLevel::Player));
        assert!(acc.has_access(AccessLevel::GameMaster));
        assert!(!acc.has_access(AccessLevel::Administrator));
    }

    #[test]
    fn test_add_and_remove_character() {
        let mut acc = Account::new("user", "pass");
        assert_eq!(acc.add_character("Warrior".to_string()), Ok(0));
        assert_eq!(acc.add_character("Mage".to_string()), Ok(1));
        assert!(acc.remove_character("Warrior"));
        assert!(!acc.remove_character("Warrior")); // already removed
        // Slot 0 should be free again
        assert_eq!(acc.add_character("Archer".to_string()), Ok(0));
    }

    #[test]
    fn test_character_slots_full() {
        let mut acc = Account::new("user", "pass");
        for i in 0..7 {
            assert!(acc.add_character(format!("Char{}", i)).is_ok());
        }
        assert!(acc.add_character("Extra".to_string()).is_err());
    }

    #[test]
    fn test_ban_unban() {
        let mut acc = Account::new("user", "pass");
        acc.ban("cheating");
        assert!(acc.banned);
        assert_eq!(acc.ban_reason.as_deref(), Some("cheating"));
        acc.unban();
        assert!(!acc.banned);
        assert!(acc.ban_reason.is_none());
    }
}
