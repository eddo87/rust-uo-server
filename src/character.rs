use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Stats & derived stats
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stats {
    pub strength: i16,
    pub dexterity: i16,
    pub intelligence: i16,
}

impl Stats {
    pub fn total(&self) -> i16 {
        self.strength + self.dexterity + self.intelligence
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivedStats {
    pub hit_points: StatPair,
    pub stamina: StatPair,
    pub mana: StatPair,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatPair {
    pub current: i16,
    pub max: i16,
}

impl StatPair {
    pub fn new(max: i16) -> Self {
        Self { current: max, max }
    }
}

// ---------------------------------------------------------------------------
// Skill system
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Skill {
    Alchemy,
    Anatomy,
    AnimalLore,
    AnimalTaming,
    Archery,
    ArmsLore,
    Begging,
    Blacksmithy,
    Bowcraft,
    Bushido,
    Camping,
    Carpentry,
    Cartography,
    Chivalry,
    Cooking,
    DetectHidden,
    Discordance,
    EvalInt,
    Fencing,
    Fishing,
    Focus,
    Forensics,
    Healing,
    Herding,
    Hiding,
    Imbuing,
    Inscription,
    ItemID,
    Lockpicking,
    Lumberjacking,
    Macing,
    Magery,
    Meditation,
    Mining,
    Musicianship,
    Mysticism,
    Necromancy,
    Ninjitsu,
    Parrying,
    Peacemaking,
    Poisoning,
    Provocation,
    RemoveTrap,
    Resisting,
    Snooping,
    Spellweaving,
    SpiritSpeak,
    Stealing,
    Stealth,
    Swordsmanship,
    Tactics,
    Tailoring,
    TasteID,
    Tinkering,
    Throwing,
    Tracking,
    Veterinary,
    Wrestling,
}

impl Skill {
    /// Returns a slice of every skill variant, useful for iteration.
    pub fn all() -> &'static [Skill] {
        use Skill::*;
        &[
            Alchemy,
            Anatomy,
            AnimalLore,
            AnimalTaming,
            Archery,
            ArmsLore,
            Begging,
            Blacksmithy,
            Bowcraft,
            Bushido,
            Camping,
            Carpentry,
            Cartography,
            Chivalry,
            Cooking,
            DetectHidden,
            Discordance,
            EvalInt,
            Fencing,
            Fishing,
            Focus,
            Forensics,
            Healing,
            Herding,
            Hiding,
            Imbuing,
            Inscription,
            ItemID,
            Lockpicking,
            Lumberjacking,
            Macing,
            Magery,
            Meditation,
            Mining,
            Musicianship,
            Mysticism,
            Necromancy,
            Ninjitsu,
            Parrying,
            Peacemaking,
            Poisoning,
            Provocation,
            RemoveTrap,
            Resisting,
            Snooping,
            Spellweaving,
            SpiritSpeak,
            Stealing,
            Stealth,
            Swordsmanship,
            Tactics,
            Tailoring,
            TasteID,
            Tinkering,
            Throwing,
            Tracking,
            Veterinary,
            Wrestling,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillLock {
    Up,
    Down,
    Locked,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SkillEntry {
    /// Current skill value (0.0 - 120.0).
    pub value: f32,
    /// Individual skill cap (typically 100.0 or 120.0).
    pub cap: f32,
    /// Lock direction for automatic gain/loss.
    pub lock: SkillLock,
}

impl SkillEntry {
    pub fn new() -> Self {
        Self {
            value: 0.0,
            cap: 100.0,
            lock: SkillLock::Up,
        }
    }
}

// ---------------------------------------------------------------------------
// Character
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub serial: u32,
    pub name: String,
    pub title: Option<String>,
    /// Body graphic id (e.g. 0x0190 for male human, 0x0191 for female human).
    pub body_type: u16,
    /// Skin hue.
    pub hue: u16,
    pub stats: Stats,
    pub derived_stats: DerivedStats,
    pub skills: HashMap<Skill, SkillEntry>,
    /// World position (x, y, z).
    pub position: (u16, u16, i8),
    /// Facing direction (0-7 in UO).
    pub direction: u8,
    /// Map/facet index (0 = Felucca, 1 = Trammel, etc.).
    pub map: u8,
    pub gold: u32,
    pub karma: i32,
    pub fame: i32,
    pub is_alive: bool,
    pub is_criminal: bool,
}

impl Character {
    /// Create a new character with sensible UO defaults.
    ///
    /// The character starts at the classic Britain bank position with base
    /// stats of 10/10/10, 50 HP/stamina/mana, and all skills at 0.0.
    pub fn new_default(serial: u32, name: String) -> Self {
        let stats = Stats {
            strength: 10,
            dexterity: 10,
            intelligence: 10,
        };

        let derived_stats = DerivedStats {
            hit_points: StatPair::new(50),
            stamina: StatPair::new(50),
            mana: StatPair::new(50),
        };

        let mut skills = HashMap::new();
        for &skill in Skill::all() {
            skills.insert(skill, SkillEntry::new());
        }

        Self {
            serial,
            name,
            title: None,
            body_type: 0x0190, // male human
            hue: 0x0000,
            stats,
            derived_stats,
            skills,
            position: (1496, 1628, 10), // Britain bank
            direction: 0,
            map: 0, // Felucca
            gold: 100,
            karma: 0,
            fame: 0,
            is_alive: true,
            is_criminal: false,
        }
    }

    /// Sum of all current skill values.
    pub fn skill_total(&self) -> f32 {
        self.skills.values().map(|e| e.value).sum()
    }

    /// Returns `true` when the character is dead.
    pub fn is_dead(&self) -> bool {
        !self.is_alive
    }

    /// Heal the character by `amount` hit points, clamped to max.
    ///
    /// Has no effect on a dead character.
    pub fn heal(&mut self, amount: i16) {
        if !self.is_alive {
            return;
        }
        let hp = &mut self.derived_stats.hit_points;
        hp.current = (hp.current + amount).min(hp.max);
    }

    /// Apply `amount` damage. If hit points reach 0 the character dies.
    ///
    /// Returns the actual damage dealt (clamped so HP does not go below 0).
    pub fn damage(&mut self, amount: i16) -> i16 {
        if !self.is_alive {
            return 0;
        }
        let hp = &mut self.derived_stats.hit_points;
        let actual = amount.min(hp.current);
        hp.current -= actual;
        if hp.current <= 0 {
            self.is_alive = false;
        }
        actual
    }

    /// Attempt to gain skill points in the given skill.
    ///
    /// The gain is clamped so the skill value never exceeds its individual
    /// cap. If the skill lock is not `Up`, or the character is dead, the
    /// gain is ignored and `false` is returned. Returns `true` on success.
    pub fn gain_skill(&mut self, skill: Skill, amount: f32) -> bool {
        if !self.is_alive {
            return false;
        }

        let entry = self.skills.entry(skill).or_insert_with(SkillEntry::new);

        if entry.lock != SkillLock::Up {
            return false;
        }

        let new_value = (entry.value + amount).min(entry.cap);
        if (new_value - entry.value).abs() < f32::EPSILON {
            return false;
        }
        entry.value = new_value;
        true
    }

    /// Serialize this character to a `CharacterSave` for disk persistence.
    pub fn to_save(&self) -> crate::persistence::CharacterSave {
        let skills: HashMap<String, f32> = self.skills
            .iter()
            .map(|(skill, entry)| (format!("{:?}", skill), entry.value))
            .collect();
        crate::persistence::CharacterSave {
            name: self.name.clone(),
            serial: self.serial,
            body_type: self.body_type,
            hue: self.hue,
            strength: self.stats.strength,
            dexterity: self.stats.dexterity,
            intelligence: self.stats.intelligence,
            hit_points: self.derived_stats.hit_points.current,
            max_hit_points: self.derived_stats.hit_points.max,
            stamina: self.derived_stats.stamina.current,
            max_stamina: self.derived_stats.stamina.max,
            mana: self.derived_stats.mana.current,
            max_mana: self.derived_stats.mana.max,
            position_x: self.position.0,
            position_y: self.position.1,
            position_z: self.position.2,
            map_id: self.map,
            direction: self.direction,
            gold: self.gold,
            karma: self.karma,
            fame: self.fame,
            is_alive: self.is_alive,
            skills,
        }
    }

    /// Reconstruct a `Character` from a `CharacterSave`.
    pub fn from_save(save: &crate::persistence::CharacterSave) -> Self {
        let mut character = Character::new_default(save.serial, save.name.clone());
        character.body_type = save.body_type;
        character.hue = save.hue;
        character.stats = Stats {
            strength: save.strength,
            dexterity: save.dexterity,
            intelligence: save.intelligence,
        };
        character.derived_stats = DerivedStats {
            hit_points: StatPair { current: save.hit_points, max: save.max_hit_points },
            stamina:    StatPair { current: save.stamina,    max: save.max_stamina    },
            mana:       StatPair { current: save.mana,       max: save.max_mana       },
        };
        character.position = (save.position_x, save.position_y, save.position_z);
        character.map = save.map_id;
        character.direction = save.direction;
        character.gold = save.gold;
        character.karma = save.karma;
        character.fame = save.fame;
        character.is_alive = save.is_alive;
        // Restore skill values from string keys
        for (name, value) in &save.skills {
            // Match by Debug representation
            for &skill in Skill::all() {
                if format!("{:?}", skill) == *name {
                    if let Some(entry) = character.skills.get_mut(&skill) {
                        entry.value = *value;
                    }
                    break;
                }
            }
        }
        character
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Stats --------------------------------------------------------------

    #[test]
    fn stats_total() {
        let s = Stats {
            strength: 100,
            dexterity: 25,
            intelligence: 10,
        };
        assert_eq!(s.total(), 135);
    }

    // -- StatPair -----------------------------------------------------------

    #[test]
    fn stat_pair_new_starts_full() {
        let sp = StatPair::new(80);
        assert_eq!(sp.current, 80);
        assert_eq!(sp.max, 80);
    }

    // -- SkillEntry ---------------------------------------------------------

    #[test]
    fn skill_entry_defaults() {
        let e = SkillEntry::new();
        assert_eq!(e.value, 0.0);
        assert_eq!(e.cap, 100.0);
        assert_eq!(e.lock, SkillLock::Up);
    }

    // -- Skill::all ---------------------------------------------------------

    #[test]
    fn skill_all_contains_every_variant() {
        // There are 58 classic UO skills in this enum.
        assert_eq!(Skill::all().len(), 58);
    }

    // -- Character::new_default ---------------------------------------------

    #[test]
    fn new_default_character_is_alive() {
        let c = Character::new_default(1, "Test".into());
        assert!(c.is_alive);
        assert!(!c.is_dead());
    }

    #[test]
    fn new_default_has_all_skills() {
        let c = Character::new_default(1, "Test".into());
        assert_eq!(c.skills.len(), 58);
        for entry in c.skills.values() {
            assert_eq!(entry.value, 0.0);
        }
    }

    #[test]
    fn new_default_stats() {
        let c = Character::new_default(1, "Test".into());
        assert_eq!(c.stats.strength, 10);
        assert_eq!(c.stats.dexterity, 10);
        assert_eq!(c.stats.intelligence, 10);
    }

    #[test]
    fn new_default_derived_stats_start_full() {
        let c = Character::new_default(1, "Test".into());
        assert_eq!(c.derived_stats.hit_points.current, 50);
        assert_eq!(c.derived_stats.stamina.current, 50);
        assert_eq!(c.derived_stats.mana.current, 50);
    }

    #[test]
    fn new_default_serial_and_name() {
        let c = Character::new_default(42, "Gandalf".into());
        assert_eq!(c.serial, 42);
        assert_eq!(c.name, "Gandalf");
        assert_eq!(c.title, None);
    }

    #[test]
    fn new_default_position_and_map() {
        let c = Character::new_default(1, "Test".into());
        assert_eq!(c.position, (1496, 1628, 10));
        assert_eq!(c.map, 0);
    }

    // -- skill_total --------------------------------------------------------

    #[test]
    fn skill_total_starts_at_zero() {
        let c = Character::new_default(1, "Test".into());
        assert!((c.skill_total() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn skill_total_accumulates() {
        let mut c = Character::new_default(1, "Test".into());
        c.gain_skill(Skill::Magery, 50.0);
        c.gain_skill(Skill::Meditation, 30.0);
        assert!((c.skill_total() - 80.0).abs() < f32::EPSILON);
    }

    // -- heal ---------------------------------------------------------------

    #[test]
    fn heal_restores_hitpoints() {
        let mut c = Character::new_default(1, "Test".into());
        c.derived_stats.hit_points.current = 10;
        c.heal(20);
        assert_eq!(c.derived_stats.hit_points.current, 30);
    }

    #[test]
    fn heal_clamped_to_max() {
        let mut c = Character::new_default(1, "Test".into());
        c.derived_stats.hit_points.current = 45;
        c.heal(100);
        assert_eq!(c.derived_stats.hit_points.current, 50);
    }

    #[test]
    fn heal_does_nothing_when_dead() {
        let mut c = Character::new_default(1, "Test".into());
        c.is_alive = false;
        c.derived_stats.hit_points.current = 0;
        c.heal(50);
        assert_eq!(c.derived_stats.hit_points.current, 0);
    }

    // -- damage -------------------------------------------------------------

    #[test]
    fn damage_reduces_hitpoints() {
        let mut c = Character::new_default(1, "Test".into());
        let dealt = c.damage(10);
        assert_eq!(dealt, 10);
        assert_eq!(c.derived_stats.hit_points.current, 40);
        assert!(c.is_alive);
    }

    #[test]
    fn damage_kills_at_zero() {
        let mut c = Character::new_default(1, "Test".into());
        let dealt = c.damage(50);
        assert_eq!(dealt, 50);
        assert_eq!(c.derived_stats.hit_points.current, 0);
        assert!(!c.is_alive);
        assert!(c.is_dead());
    }

    #[test]
    fn damage_clamped_to_current_hp() {
        let mut c = Character::new_default(1, "Test".into());
        c.derived_stats.hit_points.current = 5;
        let dealt = c.damage(100);
        assert_eq!(dealt, 5);
        assert_eq!(c.derived_stats.hit_points.current, 0);
        assert!(!c.is_alive);
    }

    #[test]
    fn damage_no_effect_when_already_dead() {
        let mut c = Character::new_default(1, "Test".into());
        c.is_alive = false;
        c.derived_stats.hit_points.current = 0;
        let dealt = c.damage(10);
        assert_eq!(dealt, 0);
    }

    // -- gain_skill ---------------------------------------------------------

    #[test]
    fn gain_skill_increases_value() {
        let mut c = Character::new_default(1, "Test".into());
        assert!(c.gain_skill(Skill::Swordsmanship, 10.0));
        assert!((c.skills[&Skill::Swordsmanship].value - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn gain_skill_clamped_to_cap() {
        let mut c = Character::new_default(1, "Test".into());
        c.gain_skill(Skill::Mining, 90.0);
        c.gain_skill(Skill::Mining, 20.0);
        assert!((c.skills[&Skill::Mining].value - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn gain_skill_respects_lock_down() {
        let mut c = Character::new_default(1, "Test".into());
        c.skills.get_mut(&Skill::Hiding).unwrap().lock = SkillLock::Down;
        assert!(!c.gain_skill(Skill::Hiding, 10.0));
        assert!((c.skills[&Skill::Hiding].value - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn gain_skill_respects_lock_locked() {
        let mut c = Character::new_default(1, "Test".into());
        c.skills.get_mut(&Skill::Magery).unwrap().lock = SkillLock::Locked;
        assert!(!c.gain_skill(Skill::Magery, 5.0));
    }

    #[test]
    fn gain_skill_returns_false_when_at_cap() {
        let mut c = Character::new_default(1, "Test".into());
        c.skills.get_mut(&Skill::Tactics).unwrap().value = 100.0;
        assert!(!c.gain_skill(Skill::Tactics, 1.0));
    }

    #[test]
    fn gain_skill_no_effect_when_dead() {
        let mut c = Character::new_default(1, "Test".into());
        c.is_alive = false;
        assert!(!c.gain_skill(Skill::Alchemy, 10.0));
    }

    #[test]
    fn gain_skill_custom_cap() {
        let mut c = Character::new_default(1, "Test".into());
        c.skills.get_mut(&Skill::Magery).unwrap().cap = 120.0;
        c.gain_skill(Skill::Magery, 115.0);
        c.gain_skill(Skill::Magery, 10.0);
        assert!((c.skills[&Skill::Magery].value - 120.0).abs() < f32::EPSILON);
    }

    // -- Combined scenarios -------------------------------------------------

    #[test]
    fn damage_then_heal_cycle() {
        let mut c = Character::new_default(1, "Test".into());
        c.damage(30);
        assert_eq!(c.derived_stats.hit_points.current, 20);
        c.heal(10);
        assert_eq!(c.derived_stats.hit_points.current, 30);
        c.heal(100);
        assert_eq!(c.derived_stats.hit_points.current, 50);
    }

    #[test]
    fn multiple_skills_gain() {
        let mut c = Character::new_default(1, "Test".into());
        c.gain_skill(Skill::Magery, 80.0);
        c.gain_skill(Skill::EvalInt, 60.0);
        c.gain_skill(Skill::Meditation, 70.0);
        assert!((c.skill_total() - 210.0).abs() < f32::EPSILON);
    }
}
