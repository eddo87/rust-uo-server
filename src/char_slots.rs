use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

/// Maximum number of character slots per account (matches UO classic limit).
pub const MAX_CHARS_PER_ACCOUNT: usize = 7;

/// A character slot: either occupied or empty.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CharSlot {
    Empty,
    Occupied(CharacterData),
}

/// Full persistent data for one character.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CharacterData {
    pub name: String,
    pub serial: u32,
    pub position_x: u16,
    pub position_y: u16,
    pub position_z: i8,
    pub map_id: u8,
    pub strength: i16,
    pub dexterity: i16,
    pub intelligence: i16,
    pub hit_points: i16,
    pub max_hit_points: i16,
    pub stamina: i16,
    pub max_stamina: i16,
    pub mana: i16,
    pub max_mana: i16,
    pub gold: u32,
    /// Unix timestamp in milliseconds (0 in default/test contexts).
    pub created_at: i64,
}

impl CharacterData {
    /// Create a new character at the Britain starting position with default stats.
    ///
    /// Starting position: x=1496, y=1628, z=10, map_id=0 (Felucca).
    /// Stats: str=25, dex=25, int=10.
    /// `created_at` is set to 0 so tests remain deterministic.
    pub fn new_default(name: &str, serial: u32) -> Self {
        let strength: i16 = 25;
        let dexterity: i16 = 25;
        let intelligence: i16 = 10;

        let hp = (strength + 25).min(100);
        let stamina = (dexterity + 25).min(100);
        let mana = (intelligence + 25).min(100);

        Self {
            name: name.to_string(),
            serial,
            position_x: 1496,
            position_y: 1628,
            position_z: 10,
            map_id: 0,
            strength,
            dexterity,
            intelligence,
            hit_points: hp,
            max_hit_points: hp,
            stamina,
            max_stamina: stamina,
            mana,
            max_mana: mana,
            gold: 500,
            created_at: 0,
        }
    }
}

/// Maps lowercase account names to their character slots.
///
/// Each account has exactly `MAX_CHARS_PER_ACCOUNT` slots stored as a `Vec`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterSlots {
    /// account_name (lowercase) → fixed-length Vec of MAX_CHARS_PER_ACCOUNT slots
    accounts: HashMap<String, Vec<CharSlot>>,
    /// Monotonically increasing serial counter; starts at 1.
    next_serial: u32,
}

impl CharacterSlots {
    /// Create an empty `CharacterSlots` with the serial counter at 1.
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            next_serial: 1,
        }
    }

    /// Load from a JSON file, or return an empty `CharacterSlots` on any error.
    pub fn load_or_new(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(json) => serde_json::from_str(&json).unwrap_or_else(|_| Self::new()),
            Err(_) => Self::new(),
        }
    }

    /// Serialize and write to a JSON file.
    pub fn save_to_file(&self, path: &Path) -> io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(path, json)
    }

    /// Return an immutable reference to a character in `slot` (0-indexed) for `account`.
    ///
    /// Returns `None` if the account has no slots yet or the slot is `Empty`.
    pub fn get_character(&self, account: &str, slot: u32) -> Option<&CharacterData> {
        let key = account.to_lowercase();
        let slots = self.accounts.get(&key)?;
        let index = slot as usize;
        if index >= slots.len() {
            return None;
        }
        match &slots[index] {
            CharSlot::Occupied(data) => Some(data),
            CharSlot::Empty => None,
        }
    }

    /// Return a mutable reference to a character in `slot` (0-indexed) for `account`.
    ///
    /// Returns `None` if the account has no slots yet or the slot is `Empty`.
    pub fn get_character_mut(&mut self, account: &str, slot: u32) -> Option<&mut CharacterData> {
        let key = account.to_lowercase();
        let slots = self.accounts.get_mut(&key)?;
        let index = slot as usize;
        if index >= slots.len() {
            return None;
        }
        match &mut slots[index] {
            CharSlot::Occupied(data) => Some(data),
            CharSlot::Empty => None,
        }
    }

    /// Create a new character in the first empty slot for `account`.
    ///
    /// Returns `Ok(slot_index)` on success, or `Err` if all slots are occupied.
    pub fn create_character(&mut self, account: &str, name: &str) -> Result<u32, String> {
        let key = account.to_lowercase();

        // Ensure the account entry exists with MAX_CHARS_PER_ACCOUNT Empty slots.
        self.accounts
            .entry(key.clone())
            .or_insert_with(|| vec![CharSlot::Empty; MAX_CHARS_PER_ACCOUNT]);

        // Find the first empty slot index (immutable borrow, dropped immediately).
        let slot_index = {
            let slots = &self.accounts[&key];
            slots
                .iter()
                .position(|s| *s == CharSlot::Empty)
                .ok_or_else(|| "All character slots are full".to_string())?
        };

        // Allocate serial and build the character data before borrowing accounts again.
        let serial = self.allocate_serial();
        let data = CharacterData::new_default(name, serial);

        // The entry was just inserted above, so get_mut is guaranteed to succeed.
        if let Some(slots) = self.accounts.get_mut(&key) {
            slots[slot_index] = CharSlot::Occupied(data);
        }

        Ok(slot_index as u32)
    }

    /// List all occupied slots for `account` as `(slot_index, &CharacterData)`.
    pub fn list_characters(&self, account: &str) -> Vec<(usize, &CharacterData)> {
        let key = account.to_lowercase();
        match self.accounts.get(&key) {
            None => Vec::new(),
            Some(slots) => slots
                .iter()
                .enumerate()
                .filter_map(|(i, slot)| {
                    if let CharSlot::Occupied(data) = slot {
                        Some((i, data))
                    } else {
                        None
                    }
                })
                .collect(),
        }
    }

    /// Update the position of the character at `slot` for `account`.
    ///
    /// Does nothing if the account or slot do not exist or the slot is empty.
    pub fn update_position(
        &mut self,
        account: &str,
        slot: u32,
        x: u16,
        y: u16,
        z: i8,
        map_id: u8,
    ) {
        if let Some(data) = self.get_character_mut(account, slot) {
            data.position_x = x;
            data.position_y = y;
            data.position_z = z;
            data.map_id = map_id;
        }
    }

    /// Return the next serial and advance the counter.
    fn allocate_serial(&mut self) -> u32 {
        let serial = self.next_serial;
        self.next_serial += 1;
        serial
    }
}

impl Default for CharacterSlots {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_character_slots_is_empty() {
        let cs = CharacterSlots::new();
        assert!(cs.accounts.is_empty());
        assert_eq!(cs.next_serial, 1);
    }

    #[test]
    fn create_character_first_slot() {
        let mut cs = CharacterSlots::new();
        let slot = cs.create_character("Alice", "Warrior").expect("should create");
        assert_eq!(slot, 0);
        let data = cs.get_character("Alice", 0).expect("character should exist");
        assert_eq!(data.name, "Warrior");
        assert_eq!(data.serial, 1);
    }

    #[test]
    fn create_character_fills_empty_slots() {
        let mut cs = CharacterSlots::new();
        let s0 = cs.create_character("bob", "Fighter").expect("slot 0");
        let s1 = cs.create_character("bob", "Mage").expect("slot 1");
        let s2 = cs.create_character("bob", "Thief").expect("slot 2");
        assert_eq!(s0, 0);
        assert_eq!(s1, 1);
        assert_eq!(s2, 2);
        assert_eq!(cs.list_characters("bob").len(), 3);
    }

    #[test]
    fn create_character_returns_error_when_full() {
        let mut cs = CharacterSlots::new();
        for i in 0..MAX_CHARS_PER_ACCOUNT {
            cs.create_character("carol", &format!("Char{}", i))
                .expect("should succeed");
        }
        let result = cs.create_character("carol", "Extra");
        assert!(result.is_err(), "8th character should fail");
    }

    #[test]
    fn list_characters_returns_occupied_only() {
        let mut cs = CharacterSlots::new();
        cs.create_character("dave", "Knight").expect("create");
        cs.create_character("dave", "Wizard").expect("create");
        let chars = cs.list_characters("dave");
        assert_eq!(chars.len(), 2);
        assert_eq!(chars[0].0, 0);
        assert_eq!(chars[0].1.name, "Knight");
        assert_eq!(chars[1].0, 1);
        assert_eq!(chars[1].1.name, "Wizard");
    }

    #[test]
    fn get_character_returns_none_for_missing() {
        let cs = CharacterSlots::new();
        assert!(cs.get_character("nobody", 0).is_none());
        assert!(cs.get_character("nobody", 3).is_none());
    }

    #[test]
    fn get_character_returns_data_for_occupied() {
        let mut cs = CharacterSlots::new();
        cs.create_character("eve", "Paladin").expect("create");
        let data = cs.get_character("eve", 0).expect("should exist");
        assert_eq!(data.name, "Paladin");
        assert_eq!(data.gold, 500);
        assert_eq!(data.position_x, 1496);
        assert_eq!(data.position_y, 1628);
        assert_eq!(data.position_z, 10);
        assert_eq!(data.map_id, 0);
        assert_eq!(data.strength, 25);
        assert_eq!(data.dexterity, 25);
        assert_eq!(data.intelligence, 10);
        assert_eq!(data.hit_points, 50);
        assert_eq!(data.max_hit_points, 50);
        assert_eq!(data.stamina, 50);
        assert_eq!(data.max_stamina, 50);
        assert_eq!(data.mana, 35);
        assert_eq!(data.max_mana, 35);
    }

    #[test]
    fn update_position_changes_coords() {
        let mut cs = CharacterSlots::new();
        cs.create_character("frank", "Explorer").expect("create");
        cs.update_position("frank", 0, 2000, 3000, -5, 1);
        let data = cs.get_character("frank", 0).expect("should exist");
        assert_eq!(data.position_x, 2000);
        assert_eq!(data.position_y, 3000);
        assert_eq!(data.position_z, -5);
        assert_eq!(data.map_id, 1);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let mut cs = CharacterSlots::new();
        cs.create_character("grace", "Ranger").expect("create");
        cs.create_character("grace", "Bard").expect("create");
        cs.update_position("grace", 1, 999, 888, 5, 0);

        let dir = std::env::temp_dir();
        let path = dir.join("char_slots_test_roundtrip.json");

        cs.save_to_file(&path).expect("save should succeed");

        let loaded = CharacterSlots::load_or_new(&path);
        assert_eq!(loaded.next_serial, cs.next_serial);

        let chars = loaded.list_characters("grace");
        assert_eq!(chars.len(), 2);
        assert_eq!(chars[0].1.name, "Ranger");
        assert_eq!(chars[1].1.name, "Bard");
        assert_eq!(chars[1].1.position_x, 999);
        assert_eq!(chars[1].1.position_y, 888);
        assert_eq!(chars[1].1.position_z, 5);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_or_new_missing_file() {
        let path = Path::new("/nonexistent/path/char_slots.json");
        let cs = CharacterSlots::load_or_new(path);
        assert!(cs.accounts.is_empty());
        assert_eq!(cs.next_serial, 1);
    }
}
