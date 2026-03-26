use std::collections::HashMap;

/// The eight circles of magery, each progressively more powerful.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpellCircle {
    First,
    Second,
    Third,
    Fourth,
    Fifth,
    Sixth,
    Seventh,
    Eighth,
}

impl SpellCircle {
    /// Base mana cost for each circle (classic UO values).
    pub fn base_mana_cost(&self) -> u16 {
        match self {
            SpellCircle::First => 4,
            SpellCircle::Second => 6,
            SpellCircle::Third => 9,
            SpellCircle::Fourth => 11,
            SpellCircle::Fifth => 14,
            SpellCircle::Sixth => 20,
            SpellCircle::Seventh => 40,
            SpellCircle::Eighth => 50,
        }
    }

    /// Minimum magery skill required to cast spells in this circle.
    pub fn min_skill(&self) -> f32 {
        match self {
            SpellCircle::First => 0.0,
            SpellCircle::Second => 10.0,
            SpellCircle::Third => 20.0,
            SpellCircle::Fourth => 30.0,
            SpellCircle::Fifth => 40.0,
            SpellCircle::Sixth => 50.0,
            SpellCircle::Seventh => 60.0,
            SpellCircle::Eighth => 70.0,
        }
    }

    /// Base cast delay in milliseconds for each circle.
    pub fn base_cast_delay_ms(&self) -> u32 {
        match self {
            SpellCircle::First => 250,
            SpellCircle::Second => 500,
            SpellCircle::Third => 750,
            SpellCircle::Fourth => 1000,
            SpellCircle::Fifth => 1250,
            SpellCircle::Sixth => 1500,
            SpellCircle::Seventh => 1750,
            SpellCircle::Eighth => 2000,
        }
    }
}

/// The eight classic Ultima Online reagents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reagent {
    BlackPearl,
    Bloodmoss,
    Garlic,
    Ginseng,
    MandrakeRoot,
    Nightshade,
    SpidersSilk,
    SulfurousAsh,
}

/// Schools of magic a spell may belong to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpellSchool {
    Magery,
    Necromancy,
    Chivalry,
    Bushido,
    Ninjitsu,
    Spellweaving,
    Mysticism,
}

/// What kind of target the spell requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellTarget {
    /// No target needed (self-cast or immediate area).
    None,
    /// Target a single mobile (player or NPC).
    Mobile,
    /// Target a specific map location.
    Location,
    /// Target an item in the world.
    Item,
    /// Target either a mobile or a location.
    MobileOrLocation,
}

/// The category of effect a spell produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellEffect {
    Damage,
    Healing,
    Buff,
    Debuff,
    Summon,
    Field,
    Teleport,
    Utility,
    Transformation,
    Resurrection,
    AreaDamage,
    AreaEffect,
}

/// Full definition of a single spell.
#[derive(Debug, Clone)]
pub struct SpellDefinition {
    pub id: u16,
    pub name: String,
    pub school: SpellSchool,
    pub circle: SpellCircle,
    pub mana_cost: u16,
    pub min_skill: f32,
    pub cast_delay_ms: u32,
    pub reagents: Vec<Reagent>,
    pub target_type: SpellTarget,
    pub effect_type: SpellEffect,
}

/// Registry holding all known spell definitions, keyed by spell id.
pub struct SpellRegistry {
    spells: HashMap<u16, SpellDefinition>,
}

impl SpellRegistry {
    /// Build a new registry pre-populated with all 64 classic magery spells.
    pub fn new() -> Self {
        let mut spells = HashMap::new();

        let defs = all_magery_spells();
        for def in defs {
            spells.insert(def.id, def);
        }

        SpellRegistry { spells }
    }

    /// Look up a spell by its numeric id.
    pub fn get_spell(&self, id: u16) -> Option<&SpellDefinition> {
        self.spells.get(&id)
    }

    /// Return all spells belonging to a given circle, sorted by id.
    pub fn get_spells_by_circle(&self, circle: SpellCircle) -> Vec<&SpellDefinition> {
        let mut result: Vec<&SpellDefinition> = self
            .spells
            .values()
            .filter(|s| s.circle == circle)
            .collect();
        result.sort_by_key(|s| s.id);
        result
    }

    /// Check whether a caster meets the requirements to cast a spell.
    /// Returns Ok(()) on success or an Err describing what is missing.
    pub fn check_requirements(
        &self,
        spell_id: u16,
        caster_mana: u16,
        caster_skill: f32,
        available_reagents: &HashMap<Reagent, u16>,
    ) -> Result<(), String> {
        let spell = self
            .get_spell(spell_id)
            .ok_or_else(|| format!("Unknown spell id {}", spell_id))?;

        if caster_mana < spell.mana_cost {
            return Err(format!(
                "Not enough mana: need {}, have {}",
                spell.mana_cost, caster_mana
            ));
        }

        if caster_skill < spell.min_skill {
            return Err(format!(
                "Skill too low: need {:.1}, have {:.1}",
                spell.min_skill, caster_skill
            ));
        }

        for reagent in &spell.reagents {
            let count = available_reagents.get(reagent).copied().unwrap_or(0);
            if count == 0 {
                return Err(format!("Missing reagent: {:?}", reagent));
            }
        }

        Ok(())
    }

    /// Total number of spells registered.
    pub fn len(&self) -> usize {
        self.spells.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.spells.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Helper: build a SpellDefinition with compact syntax
// ---------------------------------------------------------------------------
fn spell(
    id: u16,
    name: &str,
    circle: SpellCircle,
    mana: u16,
    skill: f32,
    delay: u32,
    reagents: Vec<Reagent>,
    target: SpellTarget,
    effect: SpellEffect,
) -> SpellDefinition {
    SpellDefinition {
        id,
        name: name.to_string(),
        school: SpellSchool::Magery,
        circle,
        mana_cost: mana,
        min_skill: skill,
        cast_delay_ms: delay,
        reagents,
        target_type: target,
        effect_type: effect,
    }
}

/// Produce the canonical list of all 64 magery spells (ids 1..=64).
fn all_magery_spells() -> Vec<SpellDefinition> {
    use Reagent::*;
    use SpellCircle::*;
    use SpellEffect::*;
    use SpellTarget::*;

    vec![
        // ---- First Circle (ids 1-8) ----
        spell(1, "Clumsy", First, 4, 0.0, 250,
            vec![Bloodmoss, Nightshade], Mobile, Debuff),
        spell(2, "Create Food", First, 4, 0.0, 250,
            vec![Garlic, Ginseng, MandrakeRoot], None, Utility),
        spell(3, "Feeblemind", First, 4, 0.0, 250,
            vec![Nightshade, Ginseng], Mobile, Debuff),
        spell(4, "Heal", First, 4, 0.0, 250,
            vec![Garlic, Ginseng, SpidersSilk], Mobile, Healing),
        spell(5, "Magic Arrow", First, 4, 0.0, 250,
            vec![SulfurousAsh], Mobile, Damage),
        spell(6, "Night Sight", First, 4, 0.0, 250,
            vec![SpidersSilk, SulfurousAsh], Mobile, Buff),
        spell(7, "Reactive Armor", First, 4, 0.0, 250,
            vec![Garlic, SpidersSilk, SulfurousAsh], None, Buff),
        spell(8, "Weaken", First, 4, 0.0, 250,
            vec![Garlic, Nightshade], Mobile, Debuff),

        // ---- Second Circle (ids 9-16) ----
        spell(9, "Agility", Second, 6, 10.0, 500,
            vec![Bloodmoss, MandrakeRoot], Mobile, Buff),
        spell(10, "Cunning", Second, 6, 10.0, 500,
            vec![Nightshade, MandrakeRoot], Mobile, Buff),
        spell(11, "Cure", Second, 6, 10.0, 500,
            vec![Garlic, Ginseng], Mobile, Healing),
        spell(12, "Harm", Second, 6, 10.0, 500,
            vec![Nightshade, SpidersSilk], Mobile, Damage),
        spell(13, "Magic Trap", Second, 6, 10.0, 500,
            vec![Garlic, SpidersSilk, SulfurousAsh], Item, Utility),
        spell(14, "Magic Untrap", Second, 6, 10.0, 500,
            vec![Bloodmoss, SulfurousAsh], Item, Utility),
        spell(15, "Protection", Second, 6, 10.0, 500,
            vec![Garlic, Ginseng, SulfurousAsh], None, Buff),
        spell(16, "Strength", Second, 6, 10.0, 500,
            vec![MandrakeRoot, Nightshade], Mobile, Buff),

        // ---- Third Circle (ids 17-24) ----
        spell(17, "Bless", Third, 9, 20.0, 750,
            vec![Garlic, MandrakeRoot], Mobile, Buff),
        spell(18, "Fireball", Third, 9, 20.0, 750,
            vec![BlackPearl, SulfurousAsh], Mobile, Damage),
        spell(19, "Magic Lock", Third, 9, 20.0, 750,
            vec![Bloodmoss, Garlic, SulfurousAsh], Item, Utility),
        spell(20, "Poison", Third, 9, 20.0, 750,
            vec![Nightshade], Mobile, Debuff),
        spell(21, "Telekinesis", Third, 9, 20.0, 750,
            vec![Bloodmoss, MandrakeRoot], Item, Utility),
        spell(22, "Teleport", Third, 9, 20.0, 750,
            vec![Bloodmoss, MandrakeRoot], Location, Teleport),
        spell(23, "Unlock", Third, 9, 20.0, 750,
            vec![Bloodmoss, SulfurousAsh], Item, Utility),
        spell(24, "Wall of Stone", Third, 9, 20.0, 750,
            vec![Bloodmoss, Garlic], Location, Field),

        // ---- Fourth Circle (ids 25-32) ----
        spell(25, "Archcure", Fourth, 11, 30.0, 1000,
            vec![Garlic, Ginseng, MandrakeRoot], Mobile, Healing),
        spell(26, "Archprotection", Fourth, 11, 30.0, 1000,
            vec![Garlic, Ginseng, MandrakeRoot, SulfurousAsh], None, Buff),
        spell(27, "Curse", Fourth, 11, 30.0, 1000,
            vec![Garlic, Nightshade, SulfurousAsh], Mobile, Debuff),
        spell(28, "Fire Field", Fourth, 11, 30.0, 1000,
            vec![BlackPearl, SpidersSilk, SulfurousAsh], Location, Field),
        spell(29, "Greater Heal", Fourth, 11, 30.0, 1000,
            vec![Garlic, Ginseng, MandrakeRoot, SpidersSilk], Mobile, Healing),
        spell(30, "Lightning", Fourth, 11, 30.0, 1000,
            vec![MandrakeRoot, SulfurousAsh], Mobile, Damage),
        spell(31, "Mana Drain", Fourth, 11, 30.0, 1000,
            vec![BlackPearl, MandrakeRoot, SpidersSilk], Mobile, Debuff),
        spell(32, "Recall", Fourth, 11, 30.0, 1000,
            vec![BlackPearl, Bloodmoss, MandrakeRoot], Item, Teleport),

        // ---- Fifth Circle (ids 33-40) ----
        spell(33, "Blade Spirits", Fifth, 14, 40.0, 1250,
            vec![BlackPearl, MandrakeRoot, Nightshade], None, Summon),
        spell(34, "Dispel Field", Fifth, 14, 40.0, 1250,
            vec![BlackPearl, Garlic, SpidersSilk, SulfurousAsh], Location, Utility),
        spell(35, "Incognito", Fifth, 14, 40.0, 1250,
            vec![Bloodmoss, Garlic, Nightshade], None, Transformation),
        spell(36, "Magic Reflection", Fifth, 14, 40.0, 1250,
            vec![Garlic, MandrakeRoot, SpidersSilk], None, Buff),
        spell(37, "Mind Blast", Fifth, 14, 40.0, 1250,
            vec![BlackPearl, MandrakeRoot, Nightshade, SulfurousAsh], Mobile, Damage),
        spell(38, "Paralyze", Fifth, 14, 40.0, 1250,
            vec![Garlic, MandrakeRoot, SpidersSilk], Mobile, Debuff),
        spell(39, "Poison Field", Fifth, 14, 40.0, 1250,
            vec![BlackPearl, Nightshade, SpidersSilk], Location, Field),
        spell(40, "Summon Creature", Fifth, 14, 40.0, 1250,
            vec![Bloodmoss, MandrakeRoot, SpidersSilk], None, Summon),

        // ---- Sixth Circle (ids 41-48) ----
        spell(41, "Dispel", Sixth, 20, 50.0, 1500,
            vec![Garlic, MandrakeRoot, SulfurousAsh], Mobile, Utility),
        spell(42, "Energy Bolt", Sixth, 20, 50.0, 1500,
            vec![BlackPearl, Nightshade], Mobile, Damage),
        spell(43, "Explosion", Sixth, 20, 50.0, 1500,
            vec![Bloodmoss, MandrakeRoot], MobileOrLocation, AreaDamage),
        spell(44, "Invisibility", Sixth, 20, 50.0, 1500,
            vec![Bloodmoss, Nightshade], Mobile, Buff),
        spell(45, "Mark", Sixth, 20, 50.0, 1500,
            vec![BlackPearl, Bloodmoss, MandrakeRoot], Item, Utility),
        spell(46, "Mass Curse", Sixth, 20, 50.0, 1500,
            vec![Garlic, MandrakeRoot, Nightshade, SulfurousAsh], MobileOrLocation, AreaEffect),
        spell(47, "Paralyze Field", Sixth, 20, 50.0, 1500,
            vec![BlackPearl, Ginseng, SpidersSilk], Location, Field),
        spell(48, "Reveal", Sixth, 20, 50.0, 1500,
            vec![Bloodmoss, SulfurousAsh], MobileOrLocation, Utility),

        // ---- Seventh Circle (ids 49-56) ----
        spell(49, "Chain Lightning", Seventh, 40, 60.0, 1750,
            vec![BlackPearl, Bloodmoss, MandrakeRoot, SulfurousAsh], MobileOrLocation, AreaDamage),
        spell(50, "Energy Field", Seventh, 40, 60.0, 1750,
            vec![BlackPearl, MandrakeRoot, SpidersSilk, SulfurousAsh], Location, Field),
        spell(51, "Flamestrike", Seventh, 40, 60.0, 1750,
            vec![SpidersSilk, SulfurousAsh], Mobile, Damage),
        spell(52, "Gate Travel", Seventh, 40, 60.0, 1750,
            vec![BlackPearl, MandrakeRoot, SulfurousAsh], Item, Teleport),
        spell(53, "Mana Vampire", Seventh, 40, 60.0, 1750,
            vec![BlackPearl, Bloodmoss, MandrakeRoot, SpidersSilk], Mobile, Debuff),
        spell(54, "Mass Dispel", Seventh, 40, 60.0, 1750,
            vec![BlackPearl, Garlic, MandrakeRoot, SulfurousAsh], MobileOrLocation, AreaEffect),
        spell(55, "Meteor Swarm", Seventh, 40, 60.0, 1750,
            vec![Bloodmoss, MandrakeRoot, SpidersSilk, SulfurousAsh], MobileOrLocation, AreaDamage),
        spell(56, "Polymorph", Seventh, 40, 60.0, 1750,
            vec![Bloodmoss, MandrakeRoot, SpidersSilk], None, Transformation),

        // ---- Eighth Circle (ids 57-64) ----
        spell(57, "Earthquake", Eighth, 50, 70.0, 2000,
            vec![Bloodmoss, Ginseng, MandrakeRoot, SulfurousAsh], None, AreaDamage),
        spell(58, "Energy Vortex", Eighth, 50, 70.0, 2000,
            vec![BlackPearl, Bloodmoss, MandrakeRoot, Nightshade], Location, Summon),
        spell(59, "Resurrection", Eighth, 50, 70.0, 2000,
            vec![Bloodmoss, Garlic, Ginseng], Mobile, Resurrection),
        spell(60, "Air Elemental", Eighth, 50, 70.0, 2000,
            vec![Bloodmoss, MandrakeRoot, SpidersSilk], None, Summon),
        spell(61, "Summon Daemon", Eighth, 50, 70.0, 2000,
            vec![Bloodmoss, MandrakeRoot, SpidersSilk, SulfurousAsh], None, Summon),
        spell(62, "Earth Elemental", Eighth, 50, 70.0, 2000,
            vec![Bloodmoss, MandrakeRoot, SpidersSilk], None, Summon),
        spell(63, "Fire Elemental", Eighth, 50, 70.0, 2000,
            vec![Bloodmoss, MandrakeRoot, SpidersSilk, SulfurousAsh], None, Summon),
        spell(64, "Water Elemental", Eighth, 50, 70.0, 2000,
            vec![Bloodmoss, MandrakeRoot, SpidersSilk], None, Summon),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_64_spells() {
        let reg = SpellRegistry::new();
        assert_eq!(reg.len(), 64);
        assert!(!reg.is_empty());
    }

    #[test]
    fn get_spell_by_id() {
        let reg = SpellRegistry::new();

        let heal = reg.get_spell(4).expect("Heal should exist");
        assert_eq!(heal.name, "Heal");
        assert_eq!(heal.circle, SpellCircle::First);
        assert_eq!(heal.mana_cost, 4);
        assert_eq!(heal.effect_type, SpellEffect::Healing);

        let flamestrike = reg.get_spell(51).expect("Flamestrike should exist");
        assert_eq!(flamestrike.name, "Flamestrike");
        assert_eq!(flamestrike.circle, SpellCircle::Seventh);
        assert_eq!(flamestrike.mana_cost, 40);
    }

    #[test]
    fn get_spell_unknown_id_returns_none() {
        let reg = SpellRegistry::new();
        assert!(reg.get_spell(999).is_none());
        assert!(reg.get_spell(0).is_none());
    }

    #[test]
    fn each_circle_has_eight_spells() {
        let reg = SpellRegistry::new();
        let circles = [
            SpellCircle::First,
            SpellCircle::Second,
            SpellCircle::Third,
            SpellCircle::Fourth,
            SpellCircle::Fifth,
            SpellCircle::Sixth,
            SpellCircle::Seventh,
            SpellCircle::Eighth,
        ];
        for circle in &circles {
            let spells = reg.get_spells_by_circle(*circle);
            assert_eq!(
                spells.len(),
                8,
                "Circle {:?} should have 8 spells but has {}",
                circle,
                spells.len()
            );
        }
    }

    #[test]
    fn spells_by_circle_sorted_by_id() {
        let reg = SpellRegistry::new();
        let first_circle = reg.get_spells_by_circle(SpellCircle::First);
        let ids: Vec<u16> = first_circle.iter().map(|s| s.id).collect();
        assert_eq!(ids, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn check_requirements_pass() {
        let reg = SpellRegistry::new();
        let mut reagents = HashMap::new();
        reagents.insert(Reagent::Garlic, 10);
        reagents.insert(Reagent::Ginseng, 10);
        reagents.insert(Reagent::SpidersSilk, 10);

        // Heal (id 4): mana 4, skill 0.0, reagents: Garlic, Ginseng, SpidersSilk
        let result = reg.check_requirements(4, 50, 100.0, &reagents);
        assert!(result.is_ok());
    }

    #[test]
    fn check_requirements_not_enough_mana() {
        let reg = SpellRegistry::new();
        let mut reagents = HashMap::new();
        reagents.insert(Reagent::Garlic, 10);
        reagents.insert(Reagent::Ginseng, 10);
        reagents.insert(Reagent::SpidersSilk, 10);

        let result = reg.check_requirements(4, 2, 100.0, &reagents);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("mana"));
    }

    #[test]
    fn check_requirements_skill_too_low() {
        let reg = SpellRegistry::new();
        let mut reagents = HashMap::new();
        reagents.insert(Reagent::BlackPearl, 10);
        reagents.insert(Reagent::Bloodmoss, 10);
        reagents.insert(Reagent::MandrakeRoot, 10);
        reagents.insert(Reagent::SulfurousAsh, 10);

        // Chain Lightning (id 49): circle Seventh, min_skill 60.0
        let result = reg.check_requirements(49, 100, 30.0, &reagents);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Skill"));
    }

    #[test]
    fn check_requirements_missing_reagent() {
        let reg = SpellRegistry::new();
        let reagents = HashMap::new(); // empty

        let result = reg.check_requirements(4, 50, 100.0, &reagents);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("reagent"));
    }

    #[test]
    fn check_requirements_unknown_spell() {
        let reg = SpellRegistry::new();
        let reagents = HashMap::new();

        let result = reg.check_requirements(999, 50, 100.0, &reagents);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown spell"));
    }

    #[test]
    fn spell_circle_base_values() {
        assert_eq!(SpellCircle::First.base_mana_cost(), 4);
        assert_eq!(SpellCircle::Eighth.base_mana_cost(), 50);
        assert_eq!(SpellCircle::First.min_skill(), 0.0);
        assert_eq!(SpellCircle::Eighth.min_skill(), 70.0);
        assert_eq!(SpellCircle::First.base_cast_delay_ms(), 250);
        assert_eq!(SpellCircle::Eighth.base_cast_delay_ms(), 2000);
    }

    #[test]
    fn all_spell_ids_are_unique() {
        let defs = all_magery_spells();
        let mut seen = std::collections::HashSet::new();
        for d in &defs {
            assert!(
                seen.insert(d.id),
                "Duplicate spell id {} ({})",
                d.id,
                d.name
            );
        }
    }

    #[test]
    fn all_spells_are_magery_school() {
        let reg = SpellRegistry::new();
        for id in 1..=64u16 {
            let spell = reg.get_spell(id).unwrap();
            assert_eq!(spell.school, SpellSchool::Magery);
        }
    }

    #[test]
    fn spell_ids_sequential_1_through_64() {
        let reg = SpellRegistry::new();
        for id in 1..=64u16 {
            assert!(
                reg.get_spell(id).is_some(),
                "Spell id {} should exist",
                id
            );
        }
    }

    #[test]
    fn eighth_circle_contains_expected_spells() {
        let reg = SpellRegistry::new();
        let eighth = reg.get_spells_by_circle(SpellCircle::Eighth);
        let names: Vec<&str> = eighth.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Earthquake"));
        assert!(names.contains(&"Energy Vortex"));
        assert!(names.contains(&"Resurrection"));
        assert!(names.contains(&"Summon Daemon"));
    }

    #[test]
    fn reagent_requirements_for_fireball() {
        let reg = SpellRegistry::new();
        let fireball = reg.get_spell(18).expect("Fireball should exist");
        assert_eq!(fireball.name, "Fireball");
        assert!(fireball.reagents.contains(&Reagent::BlackPearl));
        assert!(fireball.reagents.contains(&Reagent::SulfurousAsh));
        assert_eq!(fireball.reagents.len(), 2);
    }

    #[test]
    fn check_requirements_exact_mana_passes() {
        let reg = SpellRegistry::new();
        let mut reagents = HashMap::new();
        reagents.insert(Reagent::SulfurousAsh, 1);

        // Magic Arrow (id 5): mana 4, skill 0.0, reagents: SulfurousAsh
        let result = reg.check_requirements(5, 4, 0.0, &reagents);
        assert!(result.is_ok(), "Exact mana should be sufficient");
    }
}
