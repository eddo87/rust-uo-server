/// The type classification for a mobile entity in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobileType {
    Player,
    NPC,
    Monster,
    Animal,
    Vendor,
    Guard,
    Healer,
}

/// Notoriety determines the highlight color shown to other players and
/// governs PvP flagging rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Notoriety {
    /// Blue — an innocent, non-criminal entity.
    Innocent,
    /// Gray — a criminal or aggressor.
    Criminal,
    /// Red — a player-killer / murderer.
    Murderer,
    /// Orange — an enemy (war/faction).
    Enemy,
    /// Green — an ally (guild/faction).
    Ally,
    /// Yellow — an invulnerable NPC (vendors, quest givers, etc.).
    Invulnerable,
    /// Green with guild tag — same-guild member.
    Guild,
}

/// Artificial-intelligence behaviour preset for NPCs and monsters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiType {
    Melee,
    Archer,
    Mage,
    Healer,
    Vendor,
    Guard,
    Animal,
    Passive,
}

/// A mobile is any moving entity in the game world — players, NPCs,
/// monsters, animals, vendors, guards, and healers all share this
/// representation.
#[derive(Debug, Clone)]
pub struct Mobile {
    pub serial: u32,
    pub name: String,
    pub body_type: u16,
    pub hue: u16,
    pub mobile_type: MobileType,
    pub position: (u16, u16, i8),
    pub direction: u8,
    pub map: u8,
    pub hit_points: i16,
    pub max_hit_points: i16,
    pub stamina: i16,
    pub max_stamina: i16,
    pub mana: i16,
    pub max_mana: i16,
    pub strength: i16,
    pub dexterity: i16,
    pub intelligence: i16,
    pub notoriety: Notoriety,
    pub is_alive: bool,
}

impl Mobile {
    /// Create a new mobile with the given identity fields and full
    /// health/stamina/mana derived from the base stats.
    pub fn new(
        serial: u32,
        name: String,
        body_type: u16,
        hue: u16,
        mobile_type: MobileType,
        position: (u16, u16, i8),
        direction: u8,
        map: u8,
        strength: i16,
        dexterity: i16,
        intelligence: i16,
        notoriety: Notoriety,
    ) -> Self {
        let hit_points = 50 + (strength / 2);
        let stamina = dexterity;
        let mana = intelligence;

        Mobile {
            serial,
            name,
            body_type,
            hue,
            mobile_type,
            position,
            direction,
            map,
            hit_points,
            max_hit_points: hit_points,
            stamina,
            max_stamina: stamina,
            mana,
            max_mana: mana,
            strength,
            dexterity,
            intelligence,
            notoriety,
            is_alive: true,
        }
    }

    /// Returns `true` when the mobile represents a player character.
    pub fn is_player(&self) -> bool {
        self.mobile_type == MobileType::Player
    }

    /// Apply `amount` points of damage to the mobile.
    ///
    /// Hit-points are clamped to zero; if they reach zero the mobile is
    /// killed automatically via [`Self::kill`].
    ///
    /// Returns the actual damage dealt (may be less than `amount` if the
    /// mobile had fewer hit-points remaining).
    pub fn damage(&mut self, amount: i16) -> i16 {
        if !self.is_alive || amount <= 0 {
            return 0;
        }

        let actual = amount.min(self.hit_points);
        self.hit_points -= actual;

        if self.hit_points <= 0 {
            self.kill();
        }

        actual
    }

    /// Restore `amount` hit-points, clamped to `max_hit_points`.
    ///
    /// Has no effect on a dead mobile — use [`Self::resurrect`] first.
    ///
    /// Returns the actual amount healed.
    pub fn heal(&mut self, amount: i16) -> i16 {
        if !self.is_alive || amount <= 0 {
            return 0;
        }

        let headroom = self.max_hit_points - self.hit_points;
        let actual = amount.min(headroom);
        self.hit_points += actual;
        actual
    }

    /// Kill the mobile outright, setting hit-points to zero.
    pub fn kill(&mut self) {
        self.is_alive = false;
        self.hit_points = 0;
    }

    /// Bring a dead mobile back to life with half its maximum hit-points.
    ///
    /// Returns `true` if resurrection succeeded, `false` if the mobile was
    /// already alive.
    pub fn resurrect(&mut self) -> bool {
        if self.is_alive {
            return false;
        }

        self.is_alive = true;
        self.hit_points = self.max_hit_points / 2;
        true
    }
}

/// A stat range used in [`NpcDefinition`] to randomise initial stats when
/// spawning NPC instances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatRange {
    pub min: i16,
    pub max: i16,
}

impl StatRange {
    pub fn new(min: i16, max: i16) -> Self {
        StatRange { min, max }
    }

    /// Return the midpoint of the range (used as a deterministic fallback
    /// when no RNG is available).
    pub fn midpoint(&self) -> i16 {
        (self.min + self.max) / 2
    }
}

/// A template that describes a category of NPC (e.g. "Skeleton", "Healer",
/// "Wandering Merchant").  [`NpcSpawner`] uses these definitions to create
/// concrete [`Mobile`] instances.
#[derive(Debug, Clone)]
pub struct NpcDefinition {
    pub name: String,
    pub body_type: u16,
    pub hue: u16,
    pub mobile_type: MobileType,
    pub ai_type: AiType,
    pub notoriety: Notoriety,
    pub strength: StatRange,
    pub dexterity: StatRange,
    pub intelligence: StatRange,
    pub loot_table: String,
}

/// A spawner placed in the world that creates and respawns NPC mobiles
/// according to its [`NpcDefinition`].
#[derive(Debug, Clone)]
pub struct NpcSpawner {
    pub definition: NpcDefinition,
    pub home_position: (u16, u16, i8),
    pub map: u8,
    pub wander_range: u16,
    pub respawn_delay_secs: u64,
    pub max_count: usize,
    /// Serial ids of the mobiles currently managed by this spawner.
    pub active_npcs: Vec<u32>,
}

impl NpcSpawner {
    /// Spawn a new NPC [`Mobile`] from the definition, using the stat-range
    /// midpoints for deterministic results.
    ///
    /// The caller must supply the next available `serial`.  The new mobile
    /// is recorded in `active_npcs` and returned.
    pub fn spawn(&mut self, serial: u32) -> Mobile {
        let def = &self.definition;

        let mobile = Mobile::new(
            serial,
            def.name.clone(),
            def.body_type,
            def.hue,
            def.mobile_type,
            self.home_position,
            0, // direction: north
            self.map,
            def.strength.midpoint(),
            def.dexterity.midpoint(),
            def.intelligence.midpoint(),
            def.notoriety,
        );

        self.active_npcs.push(serial);
        mobile
    }

    /// Returns `true` when the spawner has fewer active NPCs than its
    /// `max_count` and therefore should create another.
    pub fn should_respawn(&self) -> bool {
        self.active_npcs.len() < self.max_count
    }

    /// Count how many serials are still tracked as alive by the spawner.
    ///
    /// This simply returns the length of `active_npcs`; the caller is
    /// responsible for removing serials of dead mobiles via
    /// [`Self::remove_npc`].
    pub fn count_alive(&self) -> usize {
        self.active_npcs.len()
    }

    /// Remove a serial from the active list (e.g. after the mobile dies).
    ///
    /// Returns `true` if the serial was found and removed.
    pub fn remove_npc(&mut self, serial: u32) -> bool {
        if let Some(idx) = self.active_npcs.iter().position(|&s| s == serial) {
            self.active_npcs.swap_remove(idx);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // Helper builders
    // ---------------------------------------------------------------

    fn make_player(serial: u32) -> Mobile {
        Mobile::new(
            serial,
            "TestPlayer".into(),
            0x0190, // male human body
            0,
            MobileType::Player,
            (1000, 2000, 0),
            0,
            0,
            50, // str
            50, // dex
            50, // int
            Notoriety::Innocent,
        )
    }

    fn make_monster(serial: u32) -> Mobile {
        Mobile::new(
            serial,
            "Skeleton".into(),
            0x0039,
            0,
            MobileType::Monster,
            (500, 500, 0),
            0,
            0,
            80,
            60,
            20,
            Notoriety::Murderer,
        )
    }

    fn make_definition() -> NpcDefinition {
        NpcDefinition {
            name: "Skeleton".into(),
            body_type: 0x0039,
            hue: 0,
            mobile_type: MobileType::Monster,
            ai_type: AiType::Melee,
            notoriety: Notoriety::Murderer,
            strength: StatRange::new(50, 70),
            dexterity: StatRange::new(40, 60),
            intelligence: StatRange::new(10, 30),
            loot_table: "skeleton_loot".into(),
        }
    }

    fn make_spawner() -> NpcSpawner {
        NpcSpawner {
            definition: make_definition(),
            home_position: (1000, 1000, 0),
            map: 0,
            wander_range: 10,
            respawn_delay_secs: 300,
            max_count: 3,
            active_npcs: Vec::new(),
        }
    }

    // ---------------------------------------------------------------
    // Mobile::new
    // ---------------------------------------------------------------

    #[test]
    fn new_mobile_has_correct_derived_stats() {
        let m = make_player(1);
        // hp = 50 + str/2 = 50 + 25 = 75
        assert_eq!(m.hit_points, 75);
        assert_eq!(m.max_hit_points, 75);
        // stamina = dex
        assert_eq!(m.stamina, 50);
        assert_eq!(m.max_stamina, 50);
        // mana = int
        assert_eq!(m.mana, 50);
        assert_eq!(m.max_mana, 50);
        assert!(m.is_alive);
    }

    #[test]
    fn new_mobile_stores_identity_fields() {
        let m = make_player(42);
        assert_eq!(m.serial, 42);
        assert_eq!(m.name, "TestPlayer");
        assert_eq!(m.body_type, 0x0190);
        assert_eq!(m.hue, 0);
        assert_eq!(m.mobile_type, MobileType::Player);
        assert_eq!(m.position, (1000, 2000, 0));
        assert_eq!(m.direction, 0);
        assert_eq!(m.map, 0);
        assert_eq!(m.notoriety, Notoriety::Innocent);
    }

    // ---------------------------------------------------------------
    // is_player
    // ---------------------------------------------------------------

    #[test]
    fn is_player_returns_true_for_player() {
        let m = make_player(1);
        assert!(m.is_player());
    }

    #[test]
    fn is_player_returns_false_for_monster() {
        let m = make_monster(1);
        assert!(!m.is_player());
    }

    // ---------------------------------------------------------------
    // damage
    // ---------------------------------------------------------------

    #[test]
    fn damage_reduces_hit_points() {
        let mut m = make_player(1);
        let hp_before = m.hit_points;
        let dealt = m.damage(10);
        assert_eq!(dealt, 10);
        assert_eq!(m.hit_points, hp_before - 10);
        assert!(m.is_alive);
    }

    #[test]
    fn damage_kills_when_hp_reaches_zero() {
        let mut m = make_player(1);
        let hp = m.hit_points;
        let dealt = m.damage(hp);
        assert_eq!(dealt, hp);
        assert_eq!(m.hit_points, 0);
        assert!(!m.is_alive);
    }

    #[test]
    fn damage_clamps_to_remaining_hp() {
        let mut m = make_player(1);
        let hp = m.hit_points;
        let dealt = m.damage(hp + 100);
        assert_eq!(dealt, hp);
        assert_eq!(m.hit_points, 0);
        assert!(!m.is_alive);
    }

    #[test]
    fn damage_does_nothing_to_dead_mobile() {
        let mut m = make_player(1);
        m.kill();
        let dealt = m.damage(10);
        assert_eq!(dealt, 0);
        assert_eq!(m.hit_points, 0);
    }

    #[test]
    fn damage_ignores_zero_and_negative() {
        let mut m = make_player(1);
        let hp = m.hit_points;
        assert_eq!(m.damage(0), 0);
        assert_eq!(m.damage(-5), 0);
        assert_eq!(m.hit_points, hp);
    }

    // ---------------------------------------------------------------
    // heal
    // ---------------------------------------------------------------

    #[test]
    fn heal_restores_hit_points() {
        let mut m = make_player(1);
        m.damage(20);
        let healed = m.heal(10);
        assert_eq!(healed, 10);
        assert_eq!(m.hit_points, m.max_hit_points - 10);
    }

    #[test]
    fn heal_clamps_to_max_hp() {
        let mut m = make_player(1);
        m.damage(5);
        let healed = m.heal(100);
        assert_eq!(healed, 5);
        assert_eq!(m.hit_points, m.max_hit_points);
    }

    #[test]
    fn heal_does_nothing_when_full() {
        let mut m = make_player(1);
        let healed = m.heal(10);
        assert_eq!(healed, 0);
        assert_eq!(m.hit_points, m.max_hit_points);
    }

    #[test]
    fn heal_does_nothing_when_dead() {
        let mut m = make_player(1);
        m.kill();
        let healed = m.heal(50);
        assert_eq!(healed, 0);
        assert_eq!(m.hit_points, 0);
    }

    #[test]
    fn heal_ignores_zero_and_negative() {
        let mut m = make_player(1);
        m.damage(10);
        let hp = m.hit_points;
        assert_eq!(m.heal(0), 0);
        assert_eq!(m.heal(-5), 0);
        assert_eq!(m.hit_points, hp);
    }

    // ---------------------------------------------------------------
    // kill
    // ---------------------------------------------------------------

    #[test]
    fn kill_sets_dead_and_zero_hp() {
        let mut m = make_player(1);
        m.kill();
        assert!(!m.is_alive);
        assert_eq!(m.hit_points, 0);
    }

    // ---------------------------------------------------------------
    // resurrect
    // ---------------------------------------------------------------

    #[test]
    fn resurrect_revives_dead_mobile() {
        let mut m = make_player(1);
        m.kill();
        assert!(m.resurrect());
        assert!(m.is_alive);
        assert_eq!(m.hit_points, m.max_hit_points / 2);
    }

    #[test]
    fn resurrect_returns_false_if_alive() {
        let mut m = make_player(1);
        assert!(!m.resurrect());
    }

    // ---------------------------------------------------------------
    // StatRange
    // ---------------------------------------------------------------

    #[test]
    fn stat_range_midpoint() {
        let r = StatRange::new(40, 60);
        assert_eq!(r.midpoint(), 50);
    }

    #[test]
    fn stat_range_midpoint_rounds_down() {
        let r = StatRange::new(1, 4);
        assert_eq!(r.midpoint(), 2); // (1+4)/2 = 2 via integer division
    }

    // ---------------------------------------------------------------
    // NpcSpawner::spawn
    // ---------------------------------------------------------------

    #[test]
    fn spawner_creates_mobile_with_definition_values() {
        let mut spawner = make_spawner();
        let m = spawner.spawn(100);

        assert_eq!(m.serial, 100);
        assert_eq!(m.name, "Skeleton");
        assert_eq!(m.body_type, 0x0039);
        assert_eq!(m.mobile_type, MobileType::Monster);
        assert_eq!(m.position, spawner.home_position);
        assert_eq!(m.map, spawner.map);
        assert_eq!(m.notoriety, Notoriety::Murderer);
        assert!(m.is_alive);
    }

    #[test]
    fn spawner_uses_stat_midpoints() {
        let mut spawner = make_spawner();
        let m = spawner.spawn(100);
        // str midpoint = (50+70)/2 = 60
        assert_eq!(m.strength, 60);
        // dex midpoint = (40+60)/2 = 50
        assert_eq!(m.dexterity, 50);
        // int midpoint = (10+30)/2 = 20
        assert_eq!(m.intelligence, 20);
    }

    #[test]
    fn spawner_tracks_serial_in_active_list() {
        let mut spawner = make_spawner();
        spawner.spawn(100);
        spawner.spawn(101);
        assert_eq!(spawner.active_npcs, vec![100, 101]);
    }

    // ---------------------------------------------------------------
    // NpcSpawner::should_respawn
    // ---------------------------------------------------------------

    #[test]
    fn should_respawn_when_below_max() {
        let mut spawner = make_spawner(); // max_count = 3
        spawner.spawn(1);
        assert!(spawner.should_respawn());
    }

    #[test]
    fn should_not_respawn_when_at_max() {
        let mut spawner = make_spawner();
        spawner.spawn(1);
        spawner.spawn(2);
        spawner.spawn(3);
        assert!(!spawner.should_respawn());
    }

    // ---------------------------------------------------------------
    // NpcSpawner::count_alive
    // ---------------------------------------------------------------

    #[test]
    fn count_alive_returns_active_count() {
        let mut spawner = make_spawner();
        assert_eq!(spawner.count_alive(), 0);
        spawner.spawn(1);
        assert_eq!(spawner.count_alive(), 1);
        spawner.spawn(2);
        assert_eq!(spawner.count_alive(), 2);
    }

    // ---------------------------------------------------------------
    // NpcSpawner::remove_npc
    // ---------------------------------------------------------------

    #[test]
    fn remove_npc_removes_serial() {
        let mut spawner = make_spawner();
        spawner.spawn(10);
        spawner.spawn(20);
        assert!(spawner.remove_npc(10));
        assert_eq!(spawner.count_alive(), 1);
        assert!(!spawner.active_npcs.contains(&10));
    }

    #[test]
    fn remove_npc_returns_false_for_unknown_serial() {
        let mut spawner = make_spawner();
        assert!(!spawner.remove_npc(999));
    }

    #[test]
    fn respawn_possible_after_removal() {
        let mut spawner = make_spawner(); // max_count = 3
        spawner.spawn(1);
        spawner.spawn(2);
        spawner.spawn(3);
        assert!(!spawner.should_respawn());
        spawner.remove_npc(2);
        assert!(spawner.should_respawn());
    }

    // ---------------------------------------------------------------
    // Integration-style: damage + kill + resurrect cycle
    // ---------------------------------------------------------------

    #[test]
    fn full_combat_lifecycle() {
        let mut m = make_monster(50);
        let max_hp = m.max_hit_points;

        // Take some damage
        m.damage(30);
        assert!(m.is_alive);
        assert_eq!(m.hit_points, max_hp - 30);

        // Heal partially
        m.heal(10);
        assert_eq!(m.hit_points, max_hp - 20);

        // Lethal damage
        m.damage(max_hp);
        assert!(!m.is_alive);
        assert_eq!(m.hit_points, 0);

        // Cannot heal while dead
        assert_eq!(m.heal(50), 0);

        // Resurrect
        assert!(m.resurrect());
        assert!(m.is_alive);
        assert_eq!(m.hit_points, max_hp / 2);

        // Heal back to full
        m.heal(max_hp);
        assert_eq!(m.hit_points, max_hp);
    }

    // ---------------------------------------------------------------
    // Enum variant coverage
    // ---------------------------------------------------------------

    #[test]
    fn mobile_type_variants_are_distinct() {
        let types = [
            MobileType::Player,
            MobileType::NPC,
            MobileType::Monster,
            MobileType::Animal,
            MobileType::Vendor,
            MobileType::Guard,
            MobileType::Healer,
        ];
        for (i, a) in types.iter().enumerate() {
            for (j, b) in types.iter().enumerate() {
                assert_eq!(a == b, i == j);
            }
        }
    }

    #[test]
    fn notoriety_variants_are_distinct() {
        let variants = [
            Notoriety::Innocent,
            Notoriety::Criminal,
            Notoriety::Murderer,
            Notoriety::Enemy,
            Notoriety::Ally,
            Notoriety::Invulnerable,
            Notoriety::Guild,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                assert_eq!(a == b, i == j);
            }
        }
    }

    #[test]
    fn ai_type_variants_are_distinct() {
        let variants = [
            AiType::Melee,
            AiType::Archer,
            AiType::Mage,
            AiType::Healer,
            AiType::Vendor,
            AiType::Guard,
            AiType::Animal,
            AiType::Passive,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                assert_eq!(a == b, i == j);
            }
        }
    }
}
