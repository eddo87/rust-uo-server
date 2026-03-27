use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::combat::ArmorStats;
use crate::movement::Position;

/// A non-player mobile in the world.
#[derive(Debug, Clone)]
pub struct Npc {
    pub serial: u32,
    pub name: String,
    /// UO body graphic ID (e.g. 0x00C9 = Dragon).
    pub body_type: u16,
    /// Skin / hue override (0 = default).
    pub hue: u16,
    pub position: Position,
    pub map_id: u8,
    /// Facing direction (0–7, UO convention).
    pub direction: u8,
    pub hit_points: i32,
    pub max_hit_points: i32,
    pub armor: ArmorStats,
    /// Defensive wrestling/combat skill used in hit-chance formula.
    pub defense_skill: f32,
    pub is_alive: bool,
    /// Notoriety byte sent in 0x78: 0x06 = red (enemy).
    pub notoriety: u8,
    /// Status flags byte sent in 0x78.
    pub flags: u8,
}

impl Npc {
    /// Create a test Dragon at the given position.
    ///
    /// Stats:
    /// - 2000 HP
    /// - Physical 40 % | Fire 100 % | Cold 20 % | Poison 40 % | Energy 40 %
    /// - Defense skill 50.0 (moderate — gives a new player ~50 % hit chance)
    pub fn dragon(serial: u32, position: Position) -> Self {
        Npc {
            serial,
            name: "Dragon".to_string(),
            body_type: 0x00C9, // classic UO dragon body
            hue: 0x0000,
            position,
            map_id: 0, // Felucca
            direction: 0x04, // facing south
            hit_points: 2000,
            max_hit_points: 2000,
            armor: ArmorStats {
                physical_resist: 40,
                fire_resist: 100,
                cold_resist: 20,
                poison_resist: 40,
                energy_resist: 40,
            },
            defense_skill: 50.0,
            is_alive: true,
            notoriety: 0x06, // red — hostile
            flags: 0x00,
        }
    }

    /// Create a Banker NPC at the given position.
    ///
    /// Bankers are blue (innocent) and open the player's bank box on double-click.
    pub fn banker(serial: u32, position: Position) -> Self {
        Npc {
            serial,
            name: "Banker".to_string(),
            body_type: 0x0190, // standard male human
            hue: 0x0000,
            position,
            map_id: 0,
            direction: 0x04,
            hit_points: 30_000,
            max_hit_points: 30_000,
            armor: ArmorStats {
                physical_resist: 0,
                fire_resist: 0,
                cold_resist: 0,
                poison_resist: 0,
                energy_resist: 0,
            },
            defense_skill: 0.0,
            is_alive: true,
            notoriety: 0x01, // innocent (blue)
            flags: 0x00,
        }
    }
}

// ---------------------------------------------------------------------------
// NpcManager
// ---------------------------------------------------------------------------

/// Thread-safe registry for all active world NPCs.
///
/// Cloneable and cheap to share across tasks (Arc-backed internally).
#[derive(Debug, Clone)]
pub struct NpcManager {
    npcs: Arc<Mutex<HashMap<u32, Npc>>>,
}

impl NpcManager {
    pub fn new() -> Self {
        NpcManager { npcs: Arc::new(Mutex::new(HashMap::new())) }
    }

    /// Register an NPC in the world.  Returns its serial.
    pub fn spawn(&self, npc: Npc) -> u32 {
        let serial = npc.serial;
        self.npcs.lock().unwrap().insert(serial, npc);
        serial
    }

    /// Return a clone of the NPC with the given serial, or `None`.
    pub fn get(&self, serial: u32) -> Option<Npc> {
        self.npcs.lock().unwrap().get(&serial).cloned()
    }

    /// Apply `amount` hit points of damage.
    ///
    /// Returns `(new_hp, max_hp, died)` or `None` if serial not found / already dead.
    pub fn apply_damage(&self, serial: u32, amount: i32) -> Option<(i32, i32, bool)> {
        let mut map = self.npcs.lock().unwrap();
        let npc = map.get_mut(&serial)?;
        if !npc.is_alive {
            return None;
        }
        npc.hit_points = (npc.hit_points - amount).max(0);
        if npc.hit_points == 0 {
            npc.is_alive = false;
        }
        Some((npc.hit_points, npc.max_hit_points, !npc.is_alive))
    }

    /// Return all alive NPCs within `radius` tiles (Chebyshev) of `pos`.
    pub fn get_nearby(&self, pos: Position, radius: u16) -> Vec<Npc> {
        self.npcs
            .lock()
            .unwrap()
            .values()
            .filter(|n| n.is_alive)
            .filter(|n| {
                let dx = (n.position.x as i32 - pos.x as i32).unsigned_abs() as u16;
                let dy = (n.position.y as i32 - pos.y as i32).unsigned_abs() as u16;
                dx <= radius && dy <= radius
            })
            .cloned()
            .collect()
    }

    /// Return all alive NPCs.
    pub fn get_all(&self) -> Vec<Npc> {
        self.npcs.lock().unwrap().values().filter(|n| n.is_alive).cloned().collect()
    }

    /// Total NPC count (alive + dead).
    pub fn count(&self) -> usize {
        self.npcs.lock().unwrap().len()
    }
}

impl Default for NpcManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Serial allocation
// ---------------------------------------------------------------------------

/// First serial in the NPC range.  Player serials are 1–7 (slots 0–6),
/// so 0x0000_1000 (4096) gives ample separation.
pub const NPC_SERIAL_BASE: u32 = 0x0000_1000;

/// Serial of the Britain bank Banker NPC.
pub const BANKER_SERIAL: u32 = NPC_SERIAL_BASE + 1;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_pos() -> Position { Position { x: 1500, y: 1640, z: 10 } }

    #[test]
    fn dragon_has_correct_stats() {
        let d = Npc::dragon(NPC_SERIAL_BASE, test_pos());
        assert_eq!(d.hit_points, 2000);
        assert_eq!(d.max_hit_points, 2000);
        assert_eq!(d.armor.physical_resist, 40);
        assert_eq!(d.armor.fire_resist, 100);
        assert!(d.is_alive);
        assert_eq!(d.notoriety, 0x06);
    }

    #[test]
    fn spawn_and_get() {
        let mgr = NpcManager::new();
        let serial = mgr.spawn(Npc::dragon(NPC_SERIAL_BASE, test_pos()));
        assert_eq!(serial, NPC_SERIAL_BASE);
        assert!(mgr.get(NPC_SERIAL_BASE).is_some());
        assert!(mgr.get(NPC_SERIAL_BASE + 1).is_none());
    }

    #[test]
    fn apply_damage_reduces_hp() {
        let mgr = NpcManager::new();
        mgr.spawn(Npc::dragon(NPC_SERIAL_BASE, test_pos()));
        let (hp, max_hp, died) = mgr.apply_damage(NPC_SERIAL_BASE, 100).unwrap();
        assert_eq!(hp, 1900);
        assert_eq!(max_hp, 2000);
        assert!(!died);
    }

    #[test]
    fn apply_damage_kills_at_zero() {
        let mgr = NpcManager::new();
        mgr.spawn(Npc::dragon(NPC_SERIAL_BASE, test_pos()));
        let (hp, _, died) = mgr.apply_damage(NPC_SERIAL_BASE, 9999).unwrap();
        assert_eq!(hp, 0);
        assert!(died);
        // Dead NPC ignores further damage
        assert!(mgr.apply_damage(NPC_SERIAL_BASE, 1).is_none());
    }

    #[test]
    fn get_nearby_filters_by_radius() {
        let mgr = NpcManager::new();
        mgr.spawn(Npc::dragon(NPC_SERIAL_BASE, Position { x: 1500, y: 1640, z: 10 }));
        mgr.spawn(Npc::dragon(NPC_SERIAL_BASE + 1, Position { x: 5000, y: 5000, z: 0 }));

        let near = mgr.get_nearby(Position { x: 1500, y: 1640, z: 10 }, 18);
        assert_eq!(near.len(), 1);
        assert_eq!(near[0].serial, NPC_SERIAL_BASE);
    }

    #[test]
    fn count_includes_dead_npcs() {
        let mgr = NpcManager::new();
        mgr.spawn(Npc::dragon(NPC_SERIAL_BASE, test_pos()));
        mgr.apply_damage(NPC_SERIAL_BASE, 9999);
        assert_eq!(mgr.count(), 1); // still in registry
        assert_eq!(mgr.get_all().len(), 0); // but not alive
    }
}
