use crate::persistence::{load_from_file, save_to_file, SaveData};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct WorldState {
    inner: Arc<Mutex<WorldStateInner>>,
    save_path: PathBuf,
}

struct WorldStateInner {
    data: SaveData,
    dirty: bool,
}

impl WorldState {
    /// Loads save data from `path` if it exists; creates a default `SaveData`
    /// (with the given `shard_name`) when the file is absent.
    pub fn load_or_create(path: &Path, shard_name: &str) -> Self {
        let data = if path.exists() {
            match load_from_file(path) {
                Ok(d) => d,
                Err(e) => {
                    log::warn!("Failed to load save file {:?}: {}. Using defaults.", path, e);
                    Self::default_save_data(shard_name)
                }
            }
        } else {
            Self::default_save_data(shard_name)
        };

        WorldState {
            inner: Arc::new(Mutex::new(WorldStateInner { data, dirty: false })),
            save_path: path.to_path_buf(),
        }
    }

    /// Saves to disk only when dirty. Resets the dirty flag on success.
    pub fn save(&self) -> std::io::Result<()> {
        let mut guard = self.inner.lock().unwrap();
        if !guard.dirty {
            return Ok(());
        }
        save_to_file(&self.save_path, &guard.data)?;
        guard.dirty = false;
        Ok(())
    }

    /// Saves to disk unconditionally, regardless of the dirty flag.
    pub fn force_save(&self) -> std::io::Result<()> {
        let mut guard = self.inner.lock().unwrap();
        save_to_file(&self.save_path, &guard.data)?;
        guard.dirty = false;
        Ok(())
    }

    /// Marks the world state as having unsaved changes.
    pub fn mark_dirty(&self) {
        self.inner.lock().unwrap().dirty = true;
    }

    pub fn shard_name(&self) -> String {
        self.inner.lock().unwrap().data.shard_name.clone()
    }

    pub fn world_time(&self) -> i64 {
        self.inner.lock().unwrap().data.world_time
    }

    pub fn set_world_time(&self, t: i64) {
        let mut guard = self.inner.lock().unwrap();
        guard.data.world_time = t;
        guard.dirty = true;
    }

    pub fn character_count(&self) -> usize {
        self.inner.lock().unwrap().data.characters.len()
    }

    // ---- private helpers ----

    fn default_save_data(shard_name: &str) -> SaveData {
        SaveData {
            characters: Vec::new(),
            world_time: 0,
            shard_name: shard_name.to_string(),
        }
    }
}

// ---- tests ----

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::{save_to_file, CharacterSave, SaveData};
    use std::path::PathBuf;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("world_state_test_{}.json", name))
    }

    #[test]
    fn load_or_create_creates_default_when_missing() {
        let path = temp_path("missing");
        // Ensure the file doesn't exist
        let _ = std::fs::remove_file(&path);
        assert!(!path.exists(), "test setup: file should not exist");

        let ws = WorldState::load_or_create(&path, "DefaultShard");
        assert_eq!(ws.shard_name(), "DefaultShard");
        assert_eq!(ws.world_time(), 0);
        assert_eq!(ws.character_count(), 0);
    }

    #[test]
    fn load_or_create_loads_existing() {
        let path = temp_path("existing");
        let data = SaveData {
            characters: vec![
                CharacterSave {
                    name: "Foo".to_string(), serial: 1,
                    body_type: 0x0190, hue: 0,
                    strength: 50, dexterity: 50, intelligence: 50,
                    hit_points: 50, max_hit_points: 100,
                    stamina: 50, max_stamina: 50,
                    mana: 50, max_mana: 50,
                    position_x: 0, position_y: 0, position_z: 0,
                    map_id: 0, direction: 0,
                    gold: 0, karma: 0, fame: 0, is_alive: true,
                    skills: std::collections::HashMap::new(),
                },
                CharacterSave {
                    name: "Bar".to_string(), serial: 2,
                    body_type: 0x0190, hue: 0,
                    strength: 50, dexterity: 50, intelligence: 50,
                    hit_points: 75, max_hit_points: 100,
                    stamina: 50, max_stamina: 50,
                    mana: 50, max_mana: 50,
                    position_x: 0, position_y: 0, position_z: 0,
                    map_id: 0, direction: 0,
                    gold: 0, karma: 0, fame: 0, is_alive: true,
                    skills: std::collections::HashMap::new(),
                },
            ],
            world_time: 12345,
            shard_name: "LoadedShard".to_string(),
        };
        save_to_file(&path, &data).expect("pre-test save failed");

        let ws = WorldState::load_or_create(&path, "ShouldBeIgnored");
        assert_eq!(ws.shard_name(), "LoadedShard");
        assert_eq!(ws.world_time(), 12345);
        assert_eq!(ws.character_count(), 2);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_only_when_dirty() {
        let path = temp_path("dirty_flag");
        let _ = std::fs::remove_file(&path);

        let ws = WorldState::load_or_create(&path, "DirtyShard");

        // Not dirty: save() should be a no-op and file should not be created
        ws.save().expect("save returned an error");
        assert!(!path.exists(), "file should not be written when not dirty");

        // Mark dirty, then save — file should now be created
        ws.mark_dirty();
        ws.save().expect("save after mark_dirty failed");
        assert!(path.exists(), "file should be written after marking dirty");

        // Dirty flag should be reset; second save is a no-op
        let mtime_before = std::fs::metadata(&path)
            .expect("stat failed")
            .modified()
            .expect("mtime not supported");
        ws.save().expect("second save returned an error");
        let mtime_after = std::fs::metadata(&path)
            .expect("stat failed")
            .modified()
            .expect("mtime not supported");
        assert_eq!(
            mtime_before, mtime_after,
            "file should not be re-written on clean second save"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn force_save_always_saves() {
        let path = temp_path("force");
        let _ = std::fs::remove_file(&path);

        let ws = WorldState::load_or_create(&path, "ForceShard");

        // force_save even when not dirty
        ws.force_save().expect("force_save failed");
        assert!(path.exists(), "force_save should create the file");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn character_count_returns_correct_value() {
        let path = temp_path("char_count");
        let data = SaveData {
            characters: vec![
                CharacterSave {
                    name: "A".to_string(), serial: 1,
                    body_type: 0x0190, hue: 0,
                    strength: 50, dexterity: 50, intelligence: 50,
                    hit_points: 100, max_hit_points: 100,
                    stamina: 50, max_stamina: 50, mana: 50, max_mana: 50,
                    position_x: 0, position_y: 0, position_z: 0,
                    map_id: 0, direction: 0,
                    gold: 0, karma: 0, fame: 0, is_alive: true,
                    skills: std::collections::HashMap::new(),
                },
                CharacterSave {
                    name: "B".to_string(), serial: 2,
                    body_type: 0x0190, hue: 0,
                    strength: 50, dexterity: 50, intelligence: 50,
                    hit_points: 100, max_hit_points: 100,
                    stamina: 50, max_stamina: 50, mana: 50, max_mana: 50,
                    position_x: 0, position_y: 0, position_z: 0,
                    map_id: 0, direction: 0,
                    gold: 0, karma: 0, fame: 0, is_alive: true,
                    skills: std::collections::HashMap::new(),
                },
                CharacterSave {
                    name: "C".to_string(), serial: 3,
                    body_type: 0x0190, hue: 0,
                    strength: 50, dexterity: 50, intelligence: 50,
                    hit_points: 100, max_hit_points: 100,
                    stamina: 50, max_stamina: 50, mana: 50, max_mana: 50,
                    position_x: 0, position_y: 0, position_z: 0,
                    map_id: 0, direction: 0,
                    gold: 0, karma: 0, fame: 0, is_alive: true,
                    skills: std::collections::HashMap::new(),
                },
            ],
            world_time: 0,
            shard_name: "CountShard".to_string(),
        };
        save_to_file(&path, &data).expect("pre-test save failed");

        let ws = WorldState::load_or_create(&path, "CountShard");
        assert_eq!(ws.character_count(), 3);

        let _ = std::fs::remove_file(&path);
    }
}
