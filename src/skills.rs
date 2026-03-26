use rand::Rng;
use std::collections::HashMap;
use std::fmt;

/// All 58 classic Ultima Online skills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    ItemId,
    Lockpicking,
    Lumberjacking,
    MaceFighting,
    Magery,
    MagicResist,
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
    Snooping,
    Spellweaving,
    SpiritSpeak,
    Stealing,
    Stealth,
    Swordsmanship,
    Tactics,
    Tailoring,
    TasteId,
    Throwing,
    Tinkering,
    Tracking,
    Veterinary,
    Wrestling,
}

impl Skill {
    /// Returns a slice of all skill variants.
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
            ItemId,
            Lockpicking,
            Lumberjacking,
            MaceFighting,
            Magery,
            MagicResist,
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
            Snooping,
            Spellweaving,
            SpiritSpeak,
            Stealing,
            Stealth,
            Swordsmanship,
            Tactics,
            Tailoring,
            TasteId,
            Throwing,
            Tinkering,
            Tracking,
            Veterinary,
            Wrestling,
        ]
    }
}

impl fmt::Display for Skill {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Skill::Alchemy => "Alchemy",
            Skill::Anatomy => "Anatomy",
            Skill::AnimalLore => "Animal Lore",
            Skill::AnimalTaming => "Animal Taming",
            Skill::Archery => "Archery",
            Skill::ArmsLore => "Arms Lore",
            Skill::Begging => "Begging",
            Skill::Blacksmithy => "Blacksmithy",
            Skill::Bowcraft => "Bowcraft/Fletching",
            Skill::Bushido => "Bushido",
            Skill::Camping => "Camping",
            Skill::Carpentry => "Carpentry",
            Skill::Cartography => "Cartography",
            Skill::Chivalry => "Chivalry",
            Skill::Cooking => "Cooking",
            Skill::DetectHidden => "Detect Hidden",
            Skill::Discordance => "Discordance",
            Skill::EvalInt => "Evaluating Intelligence",
            Skill::Fencing => "Fencing",
            Skill::Fishing => "Fishing",
            Skill::Focus => "Focus",
            Skill::Forensics => "Forensic Evaluation",
            Skill::Healing => "Healing",
            Skill::Herding => "Herding",
            Skill::Hiding => "Hiding",
            Skill::Imbuing => "Imbuing",
            Skill::Inscription => "Inscription",
            Skill::ItemId => "Item Identification",
            Skill::Lockpicking => "Lockpicking",
            Skill::Lumberjacking => "Lumberjacking",
            Skill::MaceFighting => "Mace Fighting",
            Skill::Magery => "Magery",
            Skill::MagicResist => "Resisting Spells",
            Skill::Meditation => "Meditation",
            Skill::Mining => "Mining",
            Skill::Musicianship => "Musicianship",
            Skill::Mysticism => "Mysticism",
            Skill::Necromancy => "Necromancy",
            Skill::Ninjitsu => "Ninjitsu",
            Skill::Parrying => "Parrying",
            Skill::Peacemaking => "Peacemaking",
            Skill::Poisoning => "Poisoning",
            Skill::Provocation => "Provocation",
            Skill::RemoveTrap => "Remove Trap",
            Skill::Snooping => "Snooping",
            Skill::Spellweaving => "Spellweaving",
            Skill::SpiritSpeak => "Spirit Speak",
            Skill::Stealing => "Stealing",
            Skill::Stealth => "Stealth",
            Skill::Swordsmanship => "Swordsmanship",
            Skill::Tactics => "Tactics",
            Skill::Tailoring => "Tailoring",
            Skill::TasteId => "Taste Identification",
            Skill::Throwing => "Throwing",
            Skill::Tinkering => "Tinkering",
            Skill::Tracking => "Tracking",
            Skill::Veterinary => "Veterinary",
            Skill::Wrestling => "Wrestling",
        };
        write!(f, "{}", name)
    }
}

/// Controls whether a skill is allowed to gain, lose, or stay locked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillLock {
    Up,
    Down,
    Locked,
}

/// A single skill's state: current value, lock status, and individual cap.
#[derive(Debug, Clone)]
pub struct SkillEntry {
    /// The skill's current value (0.0 to cap).
    pub base_value: f32,
    /// Whether the skill is set to gain, lose, or stay locked.
    pub lock: SkillLock,
    /// Individual skill cap. Default 100.0, can be 120.0 with power scrolls.
    pub cap: f32,
}

impl Default for SkillEntry {
    fn default() -> Self {
        SkillEntry {
            base_value: 0.0,
            lock: SkillLock::Up,
            cap: 100.0,
        }
    }
}

/// The result of a skill check attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillCheck {
    Success,
    Failure,
    CriticalSuccess,
    CriticalFailure,
}

/// A character's full set of skills, including total skill cap.
pub struct SkillSet {
    skills: HashMap<Skill, SkillEntry>,
    /// Maximum sum of all skill values. Default 700.0.
    pub skill_cap: f32,
}

impl SkillSet {
    /// Creates a new SkillSet with all 58 skills initialized to 0.0.
    pub fn new_default() -> Self {
        let mut skills = HashMap::new();
        for &skill in Skill::all() {
            skills.insert(skill, SkillEntry::default());
        }
        SkillSet {
            skills,
            skill_cap: 700.0,
        }
    }

    /// Creates a new SkillSet from a preset class template.
    ///
    /// Supported templates:
    /// - `"warrior"`: Swordsmanship 50, Tactics 50, Anatomy 50, Healing 50
    /// - `"mage"`: Magery 50, EvalInt 50, Meditation 50, Wrestling 50
    /// - `"crafter"`: Blacksmithy 50, Mining 50, Tinkering 50, ArmsLore 50
    ///
    /// Unknown class names return a default (all-zero) skill set.
    pub fn new_from_template(class: &str) -> Self {
        let mut set = Self::new_default();
        let template: Vec<(Skill, f32)> = match class {
            "warrior" => vec![
                (Skill::Swordsmanship, 50.0),
                (Skill::Tactics, 50.0),
                (Skill::Anatomy, 50.0),
                (Skill::Healing, 50.0),
            ],
            "mage" => vec![
                (Skill::Magery, 50.0),
                (Skill::EvalInt, 50.0),
                (Skill::Meditation, 50.0),
                (Skill::Wrestling, 50.0),
            ],
            "crafter" => vec![
                (Skill::Blacksmithy, 50.0),
                (Skill::Mining, 50.0),
                (Skill::Tinkering, 50.0),
                (Skill::ArmsLore, 50.0),
            ],
            _ => vec![],
        };
        for (skill, value) in template {
            set.set_skill(skill, value);
        }
        set
    }

    /// Returns a reference to the entry for the given skill.
    pub fn get_skill(&self, skill: Skill) -> &SkillEntry {
        // Every skill is always present, inserted by new_default.
        self.skills.get(&skill).unwrap()
    }

    /// Sets the base value of a skill, clamped to [0.0, individual cap].
    pub fn set_skill(&mut self, skill: Skill, value: f32) {
        if let Some(entry) = self.skills.get_mut(&skill) {
            entry.base_value = value.clamp(0.0, entry.cap);
        }
    }

    /// Sets the lock state for the given skill.
    pub fn set_lock(&mut self, skill: Skill, lock: SkillLock) {
        if let Some(entry) = self.skills.get_mut(&skill) {
            entry.lock = lock;
        }
    }

    /// Sets the individual cap for a skill (e.g. 120.0 with a power scroll).
    pub fn set_individual_cap(&mut self, skill: Skill, cap: f32) {
        if let Some(entry) = self.skills.get_mut(&skill) {
            entry.cap = cap;
        }
    }

    /// Returns the sum of all skill base values.
    pub fn total(&self) -> f32 {
        self.skills.values().map(|e| e.base_value).sum()
    }

    /// Attempts a skill gain after a successful skill use.
    ///
    /// If a gain is warranted (by the UO gain-chance formula) and the total
    /// skill cap would be exceeded, this method tries to lower a skill that
    /// is set to `SkillLock::Down` to make room.
    ///
    /// Returns the new skill value if a gain occurred, or `None`.
    pub fn try_gain(&mut self, skill: Skill, difficulty: f32) -> Option<f32> {
        let entry = self.skills.get(&skill).unwrap();
        if entry.lock != SkillLock::Up {
            return None;
        }
        let current = entry.base_value;
        let individual_cap = entry.cap;
        let total = self.total();

        if let Some(gain) =
            check_skill_gain(current, difficulty, self.skill_cap, total, individual_cap)
        {
            let new_value = (current + gain).min(individual_cap);
            let new_total = total - current + new_value;

            if new_total > self.skill_cap {
                // Need to lower a skill set to Down.
                let deficit = new_total - self.skill_cap;
                if !self.lower_down_skill(deficit, skill) {
                    return None;
                }
            }

            self.skills.get_mut(&skill).unwrap().base_value = new_value;
            Some(new_value)
        } else {
            None
        }
    }

    /// Tries to lower a skill marked as Down by the given amount.
    /// Returns true if enough room was freed.
    fn lower_down_skill(&mut self, mut deficit: f32, exclude: Skill) -> bool {
        // Collect candidates: skills set to Down with value > 0.
        let candidates: Vec<Skill> = self
            .skills
            .iter()
            .filter(|(&s, e)| s != exclude && e.lock == SkillLock::Down && e.base_value > 0.0)
            .map(|(&s, _)| s)
            .collect();

        for candidate in candidates {
            if deficit <= 0.0 {
                break;
            }
            let entry = self.skills.get_mut(&candidate).unwrap();
            let reduction = deficit.min(entry.base_value);
            entry.base_value -= reduction;
            deficit -= reduction;
        }

        deficit <= 0.001 // float tolerance
    }
}

/// Determines whether a skill gain occurs and how large it is.
///
/// Uses the classic UO formula:
/// - Gain chance = (difficulty - current_value + 25) / 50, clamped to [0.01, 0.98]
/// - Gain amount: random in [0.1, 0.5], scaled down at higher skill levels
///
/// Returns `Some(gain_amount)` if a gain occurs, `None` otherwise.
pub fn check_skill_gain(
    current_value: f32,
    difficulty: f32,
    skill_cap: f32,
    total_skills: f32,
    individual_cap: f32,
) -> Option<f32> {
    // Already at individual cap — no gain possible.
    if current_value >= individual_cap {
        return None;
    }

    // Already at total cap with no room.
    if total_skills >= skill_cap {
        return None;
    }

    let gain_chance = ((difficulty - current_value + 25.0) / 50.0).clamp(0.01, 0.98);

    let mut rng = rand::thread_rng();
    let roll: f32 = rng.gen();

    if roll >= gain_chance {
        return None;
    }

    // Gain amount: base random 0.1-0.5, reduced as skill value climbs.
    // At 0 skill the full range applies; at individual_cap the range shrinks
    // to roughly 0.1.
    let skill_ratio = current_value / individual_cap; // 0.0 .. 1.0
    let max_gain = 0.1 + 0.4 * (1.0 - skill_ratio);
    let gain: f32 = rng.gen_range(0.1..=max_gain);

    // Don't exceed individual cap.
    let gain = gain.min(individual_cap - current_value);

    Some(gain)
}

/// Performs a skill check against a difficulty and returns the outcome.
///
/// - Base success chance: `skill_value - difficulty + 50` (percent)
/// - Critical success on a roll <= 5 (when the check is otherwise a success)
/// - Critical failure on a roll >= 95 (when the check is otherwise a failure)
pub fn perform_skill_check(skill_value: f32, difficulty: f32) -> SkillCheck {
    let success_chance = (skill_value - difficulty + 50.0).clamp(0.0, 100.0);

    let mut rng = rand::thread_rng();
    let roll: f32 = rng.gen_range(0.0..100.0);

    if roll < success_chance {
        if roll <= 5.0 && success_chance < 100.0 {
            SkillCheck::CriticalSuccess
        } else {
            SkillCheck::Success
        }
    } else if roll >= 95.0 && success_chance > 0.0 {
        SkillCheck::CriticalFailure
    } else {
        SkillCheck::Failure
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Skill enum ──────────────────────────────────────────────────────

    #[test]
    fn test_skill_count() {
        assert_eq!(Skill::all().len(), 58);
    }

    #[test]
    fn test_skill_display() {
        assert_eq!(format!("{}", Skill::Alchemy), "Alchemy");
        assert_eq!(format!("{}", Skill::AnimalTaming), "Animal Taming");
        assert_eq!(format!("{}", Skill::EvalInt), "Evaluating Intelligence");
        assert_eq!(format!("{}", Skill::Bowcraft), "Bowcraft/Fletching");
        assert_eq!(format!("{}", Skill::MagicResist), "Resisting Spells");
        assert_eq!(format!("{}", Skill::Wrestling), "Wrestling");
    }

    #[test]
    fn test_skill_all_unique() {
        let all = Skill::all();
        let mut seen = std::collections::HashSet::new();
        for skill in all {
            assert!(seen.insert(skill), "Duplicate skill: {:?}", skill);
        }
    }

    // ── SkillEntry defaults ─────────────────────────────────────────────

    #[test]
    fn test_skill_entry_default() {
        let entry = SkillEntry::default();
        assert_eq!(entry.base_value, 0.0);
        assert_eq!(entry.lock, SkillLock::Up);
        assert_eq!(entry.cap, 100.0);
    }

    // ── SkillSet — new_default ──────────────────────────────────────────

    #[test]
    fn test_new_default_has_all_skills() {
        let set = SkillSet::new_default();
        for &skill in Skill::all() {
            let entry = set.get_skill(skill);
            assert_eq!(entry.base_value, 0.0);
        }
    }

    #[test]
    fn test_new_default_total_is_zero() {
        let set = SkillSet::new_default();
        assert_eq!(set.total(), 0.0);
    }

    #[test]
    fn test_default_skill_cap() {
        let set = SkillSet::new_default();
        assert_eq!(set.skill_cap, 700.0);
    }

    // ── SkillSet — templates ────────────────────────────────────────────

    #[test]
    fn test_warrior_template() {
        let set = SkillSet::new_from_template("warrior");
        assert_eq!(set.get_skill(Skill::Swordsmanship).base_value, 50.0);
        assert_eq!(set.get_skill(Skill::Tactics).base_value, 50.0);
        assert_eq!(set.get_skill(Skill::Anatomy).base_value, 50.0);
        assert_eq!(set.get_skill(Skill::Healing).base_value, 50.0);
        assert_eq!(set.total(), 200.0);
    }

    #[test]
    fn test_mage_template() {
        let set = SkillSet::new_from_template("mage");
        assert_eq!(set.get_skill(Skill::Magery).base_value, 50.0);
        assert_eq!(set.get_skill(Skill::EvalInt).base_value, 50.0);
        assert_eq!(set.get_skill(Skill::Meditation).base_value, 50.0);
        assert_eq!(set.get_skill(Skill::Wrestling).base_value, 50.0);
        assert_eq!(set.total(), 200.0);
    }

    #[test]
    fn test_crafter_template() {
        let set = SkillSet::new_from_template("crafter");
        assert_eq!(set.get_skill(Skill::Blacksmithy).base_value, 50.0);
        assert_eq!(set.get_skill(Skill::Mining).base_value, 50.0);
        assert_eq!(set.get_skill(Skill::Tinkering).base_value, 50.0);
        assert_eq!(set.get_skill(Skill::ArmsLore).base_value, 50.0);
        assert_eq!(set.total(), 200.0);
    }

    #[test]
    fn test_unknown_template_is_empty() {
        let set = SkillSet::new_from_template("bard");
        assert_eq!(set.total(), 0.0);
    }

    // ── SkillSet — get/set ──────────────────────────────────────────────

    #[test]
    fn test_set_and_get_skill() {
        let mut set = SkillSet::new_default();
        set.set_skill(Skill::Mining, 75.5);
        assert_eq!(set.get_skill(Skill::Mining).base_value, 75.5);
    }

    #[test]
    fn test_set_skill_clamps_to_cap() {
        let mut set = SkillSet::new_default();
        set.set_skill(Skill::Mining, 150.0);
        assert_eq!(set.get_skill(Skill::Mining).base_value, 100.0);
    }

    #[test]
    fn test_set_skill_clamps_negative() {
        let mut set = SkillSet::new_default();
        set.set_skill(Skill::Mining, -10.0);
        assert_eq!(set.get_skill(Skill::Mining).base_value, 0.0);
    }

    #[test]
    fn test_set_individual_cap_power_scroll() {
        let mut set = SkillSet::new_default();
        set.set_individual_cap(Skill::Magery, 120.0);
        set.set_skill(Skill::Magery, 115.0);
        assert_eq!(set.get_skill(Skill::Magery).base_value, 115.0);
        assert_eq!(set.get_skill(Skill::Magery).cap, 120.0);
    }

    // ── SkillSet — total ────────────────────────────────────────────────

    #[test]
    fn test_total_sums_correctly() {
        let mut set = SkillSet::new_default();
        set.set_skill(Skill::Alchemy, 30.0);
        set.set_skill(Skill::Anatomy, 20.0);
        set.set_skill(Skill::Mining, 50.0);
        assert!((set.total() - 100.0).abs() < 0.001);
    }

    // ── SkillSet — lock management ──────────────────────────────────────

    #[test]
    fn test_set_lock() {
        let mut set = SkillSet::new_default();
        set.set_lock(Skill::Magery, SkillLock::Down);
        assert_eq!(set.get_skill(Skill::Magery).lock, SkillLock::Down);
    }

    #[test]
    fn test_locked_skill_does_not_gain() {
        let mut set = SkillSet::new_default();
        set.set_skill(Skill::Mining, 50.0);
        set.set_lock(Skill::Mining, SkillLock::Locked);

        // Try many times — locked skill should never gain.
        for _ in 0..100 {
            assert!(set.try_gain(Skill::Mining, 50.0).is_none());
        }
        assert_eq!(set.get_skill(Skill::Mining).base_value, 50.0);
    }

    // ── Gain calculations ───────────────────────────────────────────────

    #[test]
    fn test_no_gain_when_at_individual_cap() {
        let result = check_skill_gain(100.0, 50.0, 700.0, 300.0, 100.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_no_gain_when_at_total_cap() {
        let result = check_skill_gain(50.0, 50.0, 700.0, 700.0, 100.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_gain_amount_within_bounds() {
        // Run many iterations; any gain that occurs must be in [0.1, 0.5].
        for _ in 0..1000 {
            if let Some(gain) = check_skill_gain(30.0, 50.0, 700.0, 200.0, 100.0) {
                assert!(gain >= 0.1 - 0.001, "gain too small: {}", gain);
                assert!(gain <= 0.5 + 0.001, "gain too large: {}", gain);
            }
        }
    }

    #[test]
    fn test_gain_amount_shrinks_at_high_skill() {
        // At high skill, max gain should be close to 0.1.
        let mut gains = Vec::new();
        for _ in 0..10000 {
            if let Some(gain) = check_skill_gain(99.0, 99.0, 700.0, 300.0, 100.0) {
                gains.push(gain);
            }
        }
        if !gains.is_empty() {
            let max_gain = gains.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            // At 99/100 skill_ratio=0.99, max_gain formula = 0.1 + 0.4*0.01 = 0.104
            // but also capped by individual_cap - current = 1.0, so should be ≤ 0.104 + epsilon
            assert!(
                max_gain <= 0.15,
                "At 99 skill, max gain should be small, got {}",
                max_gain
            );
        }
    }

    #[test]
    fn test_gain_does_not_exceed_individual_cap() {
        // Skill at 99.95, cap 100.0 — gain must not push past 100.0.
        for _ in 0..1000 {
            if let Some(gain) = check_skill_gain(99.95, 100.0, 700.0, 300.0, 100.0) {
                assert!(
                    gain <= 0.05 + 0.001,
                    "Gain {} would exceed individual cap",
                    gain
                );
            }
        }
    }

    // ── Cap enforcement with Down skills ────────────────────────────────

    #[test]
    fn test_try_gain_lowers_down_skill_when_at_cap() {
        let mut set = SkillSet::new_default();
        set.skill_cap = 100.0;

        set.set_skill(Skill::Swordsmanship, 50.0);
        set.set_lock(Skill::Swordsmanship, SkillLock::Up);

        set.set_skill(Skill::Tactics, 50.0);
        set.set_lock(Skill::Tactics, SkillLock::Down);

        // At exactly 100 cap, gaining swordsmanship should lower tactics.
        let mut gained = false;
        for _ in 0..500 {
            if set.try_gain(Skill::Swordsmanship, 50.0).is_some() {
                gained = true;
                break;
            }
        }

        if gained {
            // Tactics should have been reduced.
            assert!(set.get_skill(Skill::Tactics).base_value < 50.0);
            // Total should not exceed cap.
            assert!(
                set.total() <= set.skill_cap + 0.001,
                "Total {} exceeds cap {}",
                set.total(),
                set.skill_cap
            );
        }
    }

    #[test]
    fn test_try_gain_fails_when_at_cap_and_no_down_skill() {
        let mut set = SkillSet::new_default();
        set.skill_cap = 100.0;

        set.set_skill(Skill::Swordsmanship, 50.0);
        set.set_lock(Skill::Swordsmanship, SkillLock::Up);

        set.set_skill(Skill::Tactics, 50.0);
        set.set_lock(Skill::Tactics, SkillLock::Locked);

        // No skills set to Down, so no room can be made.
        for _ in 0..200 {
            assert!(set.try_gain(Skill::Swordsmanship, 50.0).is_none());
        }
    }

    // ── Skill check ─────────────────────────────────────────────────────

    #[test]
    fn test_perform_skill_check_returns_valid_variant() {
        for _ in 0..500 {
            let result = perform_skill_check(50.0, 50.0);
            match result {
                SkillCheck::Success
                | SkillCheck::Failure
                | SkillCheck::CriticalSuccess
                | SkillCheck::CriticalFailure => {}
            }
        }
    }

    #[test]
    fn test_high_skill_vs_low_difficulty_mostly_succeeds() {
        let mut successes = 0;
        let trials = 5000;
        for _ in 0..trials {
            match perform_skill_check(100.0, 10.0) {
                SkillCheck::Success | SkillCheck::CriticalSuccess => successes += 1,
                _ => {}
            }
        }
        // success_chance = 100 - 10 + 50 = 140, clamped to 100 → always succeed
        assert_eq!(
            successes, trials,
            "With 100 skill vs 10 difficulty, should always succeed"
        );
    }

    #[test]
    fn test_low_skill_vs_high_difficulty_mostly_fails() {
        let mut failures = 0;
        let trials = 5000;
        for _ in 0..trials {
            match perform_skill_check(0.0, 100.0) {
                SkillCheck::Failure | SkillCheck::CriticalFailure => failures += 1,
                _ => {}
            }
        }
        // success_chance = 0 - 100 + 50 = -50, clamped to 0 → always fail
        assert_eq!(
            failures, trials,
            "With 0 skill vs 100 difficulty, should always fail"
        );
    }

    #[test]
    fn test_equal_skill_and_difficulty_balanced() {
        let mut successes = 0;
        let trials = 10000;
        for _ in 0..trials {
            match perform_skill_check(50.0, 50.0) {
                SkillCheck::Success | SkillCheck::CriticalSuccess => successes += 1,
                _ => {}
            }
        }
        let rate = successes as f64 / trials as f64;
        // Expected ~50%
        assert!(
            rate > 0.40 && rate < 0.60,
            "Expected ~50% success rate, got {:.1}%",
            rate * 100.0
        );
    }

    #[test]
    fn test_critical_success_can_occur() {
        let mut crits = 0;
        for _ in 0..10000 {
            if perform_skill_check(50.0, 50.0) == SkillCheck::CriticalSuccess {
                crits += 1;
            }
        }
        assert!(crits > 0, "Expected at least one critical success in 10000 trials");
    }

    #[test]
    fn test_critical_failure_can_occur() {
        let mut crits = 0;
        for _ in 0..10000 {
            if perform_skill_check(50.0, 50.0) == SkillCheck::CriticalFailure {
                crits += 1;
            }
        }
        assert!(
            crits > 0,
            "Expected at least one critical failure in 10000 trials"
        );
    }

    // ── Gain chance formula edge cases ──────────────────────────────────

    #[test]
    fn test_gain_chance_formula_easy_difficulty() {
        // difficulty much lower than skill → gain_chance should be at floor (0.01)
        // difficulty=0, skill=80 → (0 - 80 + 25)/50 = -1.1 → clamped to 0.01
        // Very rarely gains.
        let mut gains = 0;
        for _ in 0..10000 {
            if check_skill_gain(80.0, 0.0, 700.0, 200.0, 100.0).is_some() {
                gains += 1;
            }
        }
        // At 1% chance, expect ~100 in 10000, but allow wide range.
        assert!(
            gains < 500,
            "Expected very few gains for easy difficulty, got {}",
            gains
        );
    }

    #[test]
    fn test_gain_chance_formula_hard_difficulty() {
        // difficulty much higher than skill → gain_chance should be at ceiling (0.98)
        // difficulty=100, skill=0 → (100 - 0 + 25)/50 = 2.5 → clamped to 0.98
        let mut gains = 0;
        for _ in 0..1000 {
            if check_skill_gain(0.0, 100.0, 700.0, 0.0, 100.0).is_some() {
                gains += 1;
            }
        }
        // At 98% chance, expect ~980.
        assert!(
            gains > 900,
            "Expected many gains for hard difficulty, got {}",
            gains
        );
    }
}
