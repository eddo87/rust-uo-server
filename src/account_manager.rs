use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use crate::account::{Account, AccessLevel};

/// Manages all player accounts on the server.
pub struct AccountManager {
    accounts: HashMap<String, Account>,
}

/// Validate that a username is 3-16 characters long and purely alphanumeric.
fn is_valid_username(username: &str) -> bool {
    let len = username.len();
    (3..=16).contains(&len) && username.chars().all(|c| c.is_ascii_alphanumeric())
}

impl AccountManager {
    /// Create an empty account manager.
    pub fn new() -> Self {
        AccountManager {
            accounts: HashMap::new(),
        }
    }

    /// Create a new account. Returns `Err` if the username is invalid or already taken.
    pub fn create_account(&mut self, username: &str, password: &str) -> Result<(), String> {
        if !is_valid_username(username) {
            return Err(
                "Username must be 3-16 characters and contain only alphanumeric characters"
                    .to_string(),
            );
        }
        let key = username.to_lowercase();
        if self.accounts.contains_key(&key) {
            return Err("Username already exists".to_string());
        }
        let account = Account::new(username, password);
        self.accounts.insert(key, account);
        Ok(())
    }

    /// Authenticate a user by username and password.
    /// Returns `Ok(&mut Account)` on success (and updates `last_login`),
    /// or `Err` describing why authentication failed.
    pub fn authenticate(&mut self, username: &str, password: &str) -> Result<&mut Account, String> {
        let key = username.to_lowercase();
        let account = self
            .accounts
            .get_mut(&key)
            .ok_or_else(|| "Account not found".to_string())?;

        if account.banned {
            let reason = account
                .ban_reason
                .as_deref()
                .unwrap_or("No reason given");
            return Err(format!("Account is banned: {}", reason));
        }

        if !account.verify_password(password) {
            return Err("Invalid password".to_string());
        }

        account.last_login = chrono::Utc::now().timestamp();
        Ok(account)
    }

    /// Get a reference to an account by username.
    pub fn get_account(&self, username: &str) -> Option<&Account> {
        self.accounts.get(&username.to_lowercase())
    }

    /// Get a mutable reference to an account by username.
    pub fn get_account_mut(&mut self, username: &str) -> Option<&mut Account> {
        self.accounts.get_mut(&username.to_lowercase())
    }

    /// Delete an account by username. Returns `true` if it existed.
    pub fn delete_account(&mut self, username: &str) -> bool {
        self.accounts.remove(&username.to_lowercase()).is_some()
    }

    /// Persist all accounts to a JSON file.
    pub fn save_to_file(&self, path: &Path) -> io::Result<()> {
        let json = serde_json::to_string_pretty(&self.accounts)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        fs::write(path, json)
    }

    /// Load accounts from a JSON file, replacing any currently held data.
    pub fn load_from_file(&mut self, path: &Path) -> io::Result<()> {
        let data = fs::read_to_string(path)?;
        self.accounts = serde_json::from_str(&data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(())
    }

    /// Number of registered accounts.
    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    /// Whether the manager holds zero accounts.
    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }
}

impl Default for AccountManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_username_validation() {
        assert!(!is_valid_username("ab"));           // too short
        assert!(is_valid_username("abc"));           // minimum length
        assert!(is_valid_username("abcdefghijklmnop")); // 16 chars
        assert!(!is_valid_username("abcdefghijklmnopq")); // 17 chars
        assert!(!is_valid_username("user name"));    // space
        assert!(!is_valid_username("user@name"));    // special char
        assert!(is_valid_username("Player1"));       // mixed case + digit
    }

    #[test]
    fn test_create_account() {
        let mut mgr = AccountManager::new();
        assert!(mgr.create_account("Alice", "pass123").is_ok());
        assert!(mgr.create_account("Alice", "other").is_err()); // duplicate
        assert!(mgr.create_account("ab", "pass").is_err());     // too short
        assert_eq!(mgr.len(), 1);
    }

    #[test]
    fn test_authenticate_success() {
        let mut mgr = AccountManager::new();
        mgr.create_account("Bob", "secret").unwrap();
        assert!(mgr.authenticate("Bob", "secret").is_ok());
    }

    #[test]
    fn test_authenticate_wrong_password() {
        let mut mgr = AccountManager::new();
        mgr.create_account("Bob", "secret").unwrap();
        assert!(mgr.authenticate("Bob", "wrong").is_err());
    }

    #[test]
    fn test_authenticate_banned() {
        let mut mgr = AccountManager::new();
        mgr.create_account("Bob", "secret").unwrap();
        mgr.get_account_mut("Bob").unwrap().ban("cheating");
        let result = mgr.authenticate("Bob", "secret");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("banned"));
    }

    #[test]
    fn test_authenticate_not_found() {
        let mut mgr = AccountManager::new();
        assert!(mgr.authenticate("Ghost", "pass").is_err());
    }

    #[test]
    fn test_case_insensitive_lookup() {
        let mut mgr = AccountManager::new();
        mgr.create_account("Alice", "pass").unwrap();
        assert!(mgr.get_account("alice").is_some());
        assert!(mgr.get_account("ALICE").is_some());
    }

    #[test]
    fn test_delete_account() {
        let mut mgr = AccountManager::new();
        mgr.create_account("Alice", "pass").unwrap();
        assert!(mgr.delete_account("Alice"));
        assert!(!mgr.delete_account("Alice")); // already gone
        assert!(mgr.is_empty());
    }

    #[test]
    fn test_save_and_load() {
        let mut mgr = AccountManager::new();
        mgr.create_account("Alice", "pass1").unwrap();
        mgr.create_account("Bob", "pass2").unwrap();
        mgr.get_account_mut("Bob")
            .unwrap()
            .access_level = AccessLevel::GameMaster;

        let path = PathBuf::from("test_accounts.json");
        mgr.save_to_file(&path).unwrap();

        let mut mgr2 = AccountManager::new();
        mgr2.load_from_file(&path).unwrap();
        assert_eq!(mgr2.len(), 2);
        assert_eq!(
            mgr2.get_account("bob").unwrap().access_level,
            AccessLevel::GameMaster
        );
        assert!(mgr2.get_account("alice").unwrap().verify_password("pass1"));

        // Clean up
        let _ = fs::remove_file(&path);
    }
}
