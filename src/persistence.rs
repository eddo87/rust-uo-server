use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

/// Represents a single character's persistent state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CharacterSave {
    pub name: String,
    pub hitpoints: i16,
    pub max_hitpoints: i16,
    pub position_x: u16,
    pub position_y: u16,
    pub position_z: i8,
    pub map: u8,
    pub stats: (i16, i16, i16), // str, dex, int
    pub gold: u32,
    pub skills: HashMap<String, f32>,
}

/// Top-level save data containing all game state that needs to persist.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SaveData {
    pub characters: Vec<CharacterSave>,
    pub world_time: i64,
    pub shard_name: String,
}

/// Serializes save data to JSON and writes it to the given file path.
pub fn save_to_file(path: &Path, data: &SaveData) -> io::Result<()> {
    let json = serde_json::to_string_pretty(data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(path, json)
}

/// Reads JSON from the given file path and deserializes it into save data.
pub fn load_from_file(path: &Path) -> io::Result<SaveData> {
    let json = fs::read_to_string(path)?;
    serde_json::from_str(&json).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Serializes save data to a JSON string.
pub fn save_to_string(data: &SaveData) -> io::Result<String> {
    serde_json::to_string_pretty(data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Deserializes save data from a JSON string.
pub fn load_from_string(json: &str) -> io::Result<SaveData> {
    serde_json::from_str(json).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sample_save_data() -> SaveData {
        let mut skills = HashMap::new();
        skills.insert("Swordsmanship".to_string(), 85.5);
        skills.insert("Magery".to_string(), 72.0);
        skills.insert("Mining".to_string(), 100.0);

        SaveData {
            characters: vec![
                CharacterSave {
                    name: "Gandalf".to_string(),
                    hitpoints: 95,
                    max_hitpoints: 100,
                    position_x: 1602,
                    position_y: 1591,
                    position_z: 20,
                    map: 0,
                    stats: (80, 25, 45),
                    gold: 15000,
                    skills: skills.clone(),
                },
                CharacterSave {
                    name: "Frodo".to_string(),
                    hitpoints: 40,
                    max_hitpoints: 50,
                    position_x: 3000,
                    position_y: 2500,
                    position_z: -5,
                    map: 1,
                    stats: (30, 60, 10),
                    gold: 250,
                    skills: HashMap::new(),
                },
            ],
            world_time: 1700000000000,
            shard_name: "My Shard".to_string(),
        }
    }

    #[test]
    fn test_string_roundtrip() {
        let data = sample_save_data();
        let json = save_to_string(&data).expect("save_to_string should succeed");
        let loaded = load_from_string(&json).expect("load_from_string should succeed");
        assert_eq!(data, loaded);
    }

    #[test]
    fn test_file_roundtrip() {
        let data = sample_save_data();
        let dir = std::env::temp_dir();
        let path = dir.join("rust_uo_server_test_save.json");

        save_to_file(&path, &data).expect("save_to_file should succeed");
        let loaded = load_from_file(&path).expect("load_from_file should succeed");
        assert_eq!(data, loaded);

        // Clean up
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_load_missing_file() {
        let path = Path::new("/nonexistent/path/save.json");
        let result = load_from_file(path);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn test_load_corrupt_data_from_string() {
        let corrupt_json = "{ this is not valid json !!!";
        let result = load_from_string(corrupt_json);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_load_corrupt_data_from_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("rust_uo_server_test_corrupt.json");
        fs::write(&path, "not json at all").expect("writing corrupt file should succeed");

        let result = load_from_file(&path);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);

        // Clean up
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_empty_characters_roundtrip() {
        let data = SaveData {
            characters: vec![],
            world_time: 0,
            shard_name: "Empty Shard".to_string(),
        };
        let json = save_to_string(&data).expect("save_to_string should succeed");
        let loaded = load_from_string(&json).expect("load_from_string should succeed");
        assert_eq!(data, loaded);
    }

    #[test]
    fn test_negative_position_z() {
        let data = SaveData {
            characters: vec![CharacterSave {
                name: "Miner".to_string(),
                hitpoints: 100,
                max_hitpoints: 100,
                position_x: 500,
                position_y: 500,
                position_z: -128,
                map: 0,
                stats: (50, 50, 50),
                gold: 0,
                skills: HashMap::new(),
            }],
            world_time: 42,
            shard_name: "Test".to_string(),
        };
        let json = save_to_string(&data).unwrap();
        let loaded = load_from_string(&json).unwrap();
        assert_eq!(loaded.characters[0].position_z, -128);
    }
}
