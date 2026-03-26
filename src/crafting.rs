/// Crafting system for Ultima Online server.
///
/// Provides skill-based crafting with resource consumption, success chance
/// calculation, and exceptional item bonuses faithful to the classic UO model.

/// The ten craftable skill categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CraftSkill {
    Blacksmithing,
    Tailoring,
    Carpentry,
    Tinkering,
    Alchemy,
    Inscription,
    Cooking,
    Bowcraft,
    Glassblowing,
    Masonry,
}

/// Raw materials consumed by recipes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CraftResource {
    // Metals
    IronIngot,
    DullCopperIngot,
    ShadowIronIngot,
    CopperIngot,
    BronzeIngot,
    GoldIngot,
    AgapiteIngot,
    VeriteIngot,
    ValoriteIngot,
    // Leathers
    Leather,
    SpinedLeather,
    HornedLeather,
    BarbedLeather,
    // Woods
    RegularWood,
    OakWood,
    AshWood,
    YewWood,
    HeartwoodWood,
    BloodwoodWood,
    FrostwoodWood,
    // Cloth
    Cloth,
    CutCloth,
    BoltOfCloth,
}

/// Outcome of a crafting attempt.
#[derive(Debug, Clone, PartialEq)]
pub enum CraftResult {
    /// Item was crafted; `exceptional` indicates bonus quality.
    Success { item_id: u32, exceptional: bool },
    /// Skill check failed — some resources are lost.
    Failure,
    /// Character skill is below the recipe minimum.
    InsufficientSkill,
    /// Not enough materials in inventory.
    InsufficientResources { missing: Vec<(CraftResource, u32)> },
}

/// A single crafting recipe.
#[derive(Debug, Clone)]
pub struct CraftRecipe {
    pub id: u32,
    pub name: String,
    pub skill: CraftSkill,
    /// Minimum skill value (0.0–100.0) required to attempt.
    pub required_skill: f64,
    /// Resources consumed on a successful craft.
    pub resources: Vec<(CraftResource, u32)>,
    /// Base success chance when the crafter is exactly at `required_skill`.
    pub success_chance_at_min: f64,
    /// Additive bonus to the chance of producing an exceptional item.
    pub exceptional_chance_bonus: f64,
    /// Item template id produced on success.
    pub result_item_id: u32,
}

/// Central registry for recipes and craft logic.
#[derive(Debug, Default)]
pub struct CraftSystem {
    recipes: Vec<CraftRecipe>,
}

impl CraftSystem {
    pub fn new() -> Self {
        Self {
            recipes: Vec::new(),
        }
    }

    /// Create a `CraftSystem` pre-loaded with the default blacksmith recipes.
    pub fn with_defaults() -> Self {
        let mut system = Self::new();
        system.add_default_blacksmith_recipes();
        system
    }

    /// Register a new recipe. Returns the recipe id.
    pub fn add_recipe(&mut self, recipe: CraftRecipe) -> u32 {
        let id = recipe.id;
        self.recipes.push(recipe);
        id
    }

    /// Look up a recipe by its id.
    pub fn find_recipe(&self, id: u32) -> Option<&CraftRecipe> {
        self.recipes.iter().find(|r| r.id == id)
    }

    /// Look up recipes by skill category.
    pub fn recipes_for_skill(&self, skill: CraftSkill) -> Vec<&CraftRecipe> {
        self.recipes.iter().filter(|r| r.skill == skill).collect()
    }

    /// Calculate success probability for a given character skill level.
    ///
    /// Returns a value clamped to 0.0–1.0.
    /// The model is linear: at `required_skill` the chance equals
    /// `success_chance_at_min`; at 100.0 skill it reaches 1.0.
    pub fn calculate_success_chance(recipe: &CraftRecipe, skill_value: f64) -> f64 {
        if skill_value < recipe.required_skill {
            return 0.0;
        }
        let range = 100.0 - recipe.required_skill;
        if range <= 0.0 {
            return recipe.success_chance_at_min;
        }
        let progress = (skill_value - recipe.required_skill) / range;
        let chance =
            recipe.success_chance_at_min + progress * (1.0 - recipe.success_chance_at_min);
        chance.clamp(0.0, 1.0)
    }

    /// Calculate the chance to produce an exceptional item.
    ///
    /// Only meaningful when `skill_value` is above `required_skill`.
    /// Base exceptional chance starts at 0 % at minimum skill and scales
    /// linearly up to a cap, plus the recipe's `exceptional_chance_bonus`.
    pub fn calculate_exceptional_chance(recipe: &CraftRecipe, skill_value: f64) -> f64 {
        if skill_value < recipe.required_skill {
            return 0.0;
        }
        let range = 100.0 - recipe.required_skill;
        if range <= 0.0 {
            return recipe.exceptional_chance_bonus.clamp(0.0, 1.0);
        }
        let progress = (skill_value - recipe.required_skill) / range;
        let chance = progress * 0.45 + recipe.exceptional_chance_bonus;
        chance.clamp(0.0, 1.0)
    }

    /// Attempt to craft an item.
    ///
    /// * `recipe_id`   — which recipe to use.
    /// * `skill_value` — the character's current skill (0.0–100.0).
    /// * `inventory`   — mutable slice of (resource, quantity) the character
    ///   currently holds. Quantities are decremented on success.
    /// * `roll`        — a value in 0.0–1.0 representing the random outcome
    ///   (injected for deterministic testing).
    /// * `exceptional_roll` — a second 0.0–1.0 roll for exceptional quality.
    pub fn attempt_craft(
        &self,
        recipe_id: u32,
        skill_value: f64,
        inventory: &mut Vec<(CraftResource, u32)>,
        roll: f64,
        exceptional_roll: f64,
    ) -> CraftResult {
        let recipe = match self.find_recipe(recipe_id) {
            Some(r) => r.clone(),
            None => return CraftResult::Failure,
        };

        // Skill gate
        if skill_value < recipe.required_skill {
            return CraftResult::InsufficientSkill;
        }

        // Resource check
        let mut missing: Vec<(CraftResource, u32)> = Vec::new();
        for &(resource, needed) in &recipe.resources {
            let have = inventory
                .iter()
                .filter(|(r, _)| *r == resource)
                .map(|(_, q)| *q)
                .sum::<u32>();
            if have < needed {
                missing.push((resource, needed - have));
            }
        }
        if !missing.is_empty() {
            return CraftResult::InsufficientResources { missing };
        }

        // Consume resources (always consumed on attempt, even on failure)
        for &(resource, needed) in &recipe.resources {
            let mut remaining = needed;
            for (r, q) in inventory.iter_mut() {
                if *r == resource && remaining > 0 {
                    let take = remaining.min(*q);
                    *q -= take;
                    remaining -= take;
                }
            }
        }

        // Success check
        let chance = Self::calculate_success_chance(&recipe, skill_value);
        if roll >= chance {
            return CraftResult::Failure;
        }

        // Exceptional check
        let exc_chance = Self::calculate_exceptional_chance(&recipe, skill_value);
        let exceptional = exceptional_roll < exc_chance;

        CraftResult::Success {
            item_id: recipe.result_item_id,
            exceptional,
        }
    }

    // ----------------------------------------------------------------
    // Default recipe sets
    // ----------------------------------------------------------------

    fn add_default_blacksmith_recipes(&mut self) {
        // 1 – Dagger
        self.add_recipe(CraftRecipe {
            id: 1001,
            name: "Dagger".into(),
            skill: CraftSkill::Blacksmithing,
            required_skill: 0.0,
            resources: vec![(CraftResource::IronIngot, 3)],
            success_chance_at_min: 0.50,
            exceptional_chance_bonus: 0.0,
            result_item_id: 0x0F52,
        });

        // 2 – Mace
        self.add_recipe(CraftRecipe {
            id: 1002,
            name: "Mace".into(),
            skill: CraftSkill::Blacksmithing,
            required_skill: 14.5,
            resources: vec![(CraftResource::IronIngot, 6)],
            success_chance_at_min: 0.45,
            exceptional_chance_bonus: 0.0,
            result_item_id: 0x0F5C,
        });

        // 3 – Broadsword
        self.add_recipe(CraftRecipe {
            id: 1003,
            name: "Broadsword".into(),
            skill: CraftSkill::Blacksmithing,
            required_skill: 35.4,
            resources: vec![(CraftResource::IronIngot, 10)],
            success_chance_at_min: 0.40,
            exceptional_chance_bonus: 0.02,
            result_item_id: 0x0F5E,
        });

        // 4 – Platemail Chest (heavy armour)
        self.add_recipe(CraftRecipe {
            id: 1004,
            name: "Platemail Chest".into(),
            skill: CraftSkill::Blacksmithing,
            required_skill: 75.0,
            resources: vec![(CraftResource::IronIngot, 25)],
            success_chance_at_min: 0.35,
            exceptional_chance_bonus: 0.05,
            result_item_id: 0x1415,
        });

        // 5 – Chainmail Coif
        self.add_recipe(CraftRecipe {
            id: 1005,
            name: "Chainmail Coif".into(),
            skill: CraftSkill::Blacksmithing,
            required_skill: 14.5,
            resources: vec![(CraftResource::IronIngot, 10)],
            success_chance_at_min: 0.50,
            exceptional_chance_bonus: 0.0,
            result_item_id: 0x13BB,
        });

        // 6 – Viking Sword
        self.add_recipe(CraftRecipe {
            id: 1006,
            name: "Viking Sword".into(),
            skill: CraftSkill::Blacksmithing,
            required_skill: 24.3,
            resources: vec![(CraftResource::IronIngot, 14)],
            success_chance_at_min: 0.45,
            exceptional_chance_bonus: 0.01,
            result_item_id: 0x13B9,
        });
    }
}

// ====================================================================
// Tests
// ====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn system() -> CraftSystem {
        CraftSystem::with_defaults()
    }

    // -- Recipe registry ---------------------------------------------------

    #[test]
    fn test_add_and_find_recipe() {
        let mut sys = CraftSystem::new();
        sys.add_recipe(CraftRecipe {
            id: 9999,
            name: "Test Sword".into(),
            skill: CraftSkill::Blacksmithing,
            required_skill: 50.0,
            resources: vec![(CraftResource::IronIngot, 5)],
            success_chance_at_min: 0.5,
            exceptional_chance_bonus: 0.0,
            result_item_id: 0xAAAA,
        });
        let r = sys.find_recipe(9999);
        assert!(r.is_some());
        assert_eq!(r.unwrap().name, "Test Sword");
    }

    #[test]
    fn test_find_recipe_missing() {
        let sys = system();
        assert!(sys.find_recipe(0).is_none());
    }

    #[test]
    fn test_default_recipes_loaded() {
        let sys = system();
        assert_eq!(sys.recipes_for_skill(CraftSkill::Blacksmithing).len(), 6);
    }

    // -- Success-chance calculation ----------------------------------------

    #[test]
    fn test_success_chance_below_min_skill() {
        let sys = system();
        let recipe = sys.find_recipe(1003).unwrap(); // Broadsword, req 35.4
        let chance = CraftSystem::calculate_success_chance(recipe, 20.0);
        assert_eq!(chance, 0.0);
    }

    #[test]
    fn test_success_chance_at_min_skill() {
        let sys = system();
        let recipe = sys.find_recipe(1003).unwrap();
        let chance = CraftSystem::calculate_success_chance(recipe, 35.4);
        assert!((chance - 0.40).abs() < 1e-9);
    }

    #[test]
    fn test_success_chance_at_max_skill() {
        let sys = system();
        let recipe = sys.find_recipe(1003).unwrap();
        let chance = CraftSystem::calculate_success_chance(recipe, 100.0);
        assert!((chance - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_success_chance_midpoint() {
        let sys = system();
        let recipe = sys.find_recipe(1001).unwrap(); // Dagger, req 0.0, min 0.50
        // At skill 50.0: progress = 0.5, chance = 0.50 + 0.5 * 0.50 = 0.75
        let chance = CraftSystem::calculate_success_chance(recipe, 50.0);
        assert!((chance - 0.75).abs() < 1e-9);
    }

    // -- Exceptional-chance calculation ------------------------------------

    #[test]
    fn test_exceptional_chance_below_min_skill() {
        let sys = system();
        let recipe = sys.find_recipe(1004).unwrap(); // Platemail, req 75.0
        let chance = CraftSystem::calculate_exceptional_chance(recipe, 50.0);
        assert_eq!(chance, 0.0);
    }

    #[test]
    fn test_exceptional_chance_at_max_skill() {
        let sys = system();
        let recipe = sys.find_recipe(1004).unwrap(); // bonus 0.05
        // progress = 1.0 => 0.45 + 0.05 = 0.50
        let chance = CraftSystem::calculate_exceptional_chance(recipe, 100.0);
        assert!((chance - 0.50).abs() < 1e-9);
    }

    // -- attempt_craft -----------------------------------------------------

    #[test]
    fn test_attempt_craft_success_normal() {
        let sys = system();
        let mut inv = vec![(CraftResource::IronIngot, 50)];
        // Dagger (1001): req 0.0, min_chance 0.50.  Skill 80 => high chance.
        // roll 0.1 is well below chance, exceptional_roll 1.0 => not exceptional
        let result = sys.attempt_craft(1001, 80.0, &mut inv, 0.1, 1.0);
        match result {
            CraftResult::Success {
                item_id,
                exceptional,
            } => {
                assert_eq!(item_id, 0x0F52);
                assert!(!exceptional);
            }
            other => panic!("Expected Success, got {:?}", other),
        }
        // 3 ingots consumed
        assert_eq!(inv[0].1, 47);
    }

    #[test]
    fn test_attempt_craft_success_exceptional() {
        let sys = system();
        let mut inv = vec![(CraftResource::IronIngot, 50)];
        // Platemail (1004): req 75.0, bonus 0.05.  Skill 100 => exc chance 0.50
        // roll 0.0 => success, exceptional_roll 0.0 => exceptional (below 0.50)
        let result = sys.attempt_craft(1004, 100.0, &mut inv, 0.0, 0.0);
        match result {
            CraftResult::Success {
                item_id,
                exceptional,
            } => {
                assert_eq!(item_id, 0x1415);
                assert!(exceptional);
            }
            other => panic!("Expected exceptional Success, got {:?}", other),
        }
        assert_eq!(inv[0].1, 25);
    }

    #[test]
    fn test_attempt_craft_failure() {
        let sys = system();
        let mut inv = vec![(CraftResource::IronIngot, 50)];
        // Dagger at skill 0 has 0.50 chance. roll 0.99 => fail
        let result = sys.attempt_craft(1001, 0.0, &mut inv, 0.99, 0.0);
        assert_eq!(result, CraftResult::Failure);
        // Resources are still consumed on failure
        assert_eq!(inv[0].1, 47);
    }

    #[test]
    fn test_attempt_craft_insufficient_skill() {
        let sys = system();
        let mut inv = vec![(CraftResource::IronIngot, 50)];
        // Platemail requires 75.0 skill
        let result = sys.attempt_craft(1004, 50.0, &mut inv, 0.0, 0.0);
        assert_eq!(result, CraftResult::InsufficientSkill);
        // No resources consumed
        assert_eq!(inv[0].1, 50);
    }

    #[test]
    fn test_attempt_craft_insufficient_resources() {
        let sys = system();
        let mut inv = vec![(CraftResource::IronIngot, 2)];
        // Dagger needs 3 iron ingots
        let result = sys.attempt_craft(1001, 80.0, &mut inv, 0.0, 0.0);
        match result {
            CraftResult::InsufficientResources { missing } => {
                assert_eq!(missing.len(), 1);
                assert_eq!(missing[0], (CraftResource::IronIngot, 1));
            }
            other => panic!("Expected InsufficientResources, got {:?}", other),
        }
        // No resources consumed
        assert_eq!(inv[0].1, 2);
    }

    #[test]
    fn test_attempt_craft_unknown_recipe() {
        let sys = system();
        let mut inv = vec![(CraftResource::IronIngot, 50)];
        let result = sys.attempt_craft(0, 100.0, &mut inv, 0.0, 0.0);
        assert_eq!(result, CraftResult::Failure);
    }

    #[test]
    fn test_recipes_for_skill_empty() {
        let sys = system();
        assert!(sys.recipes_for_skill(CraftSkill::Tailoring).is_empty());
    }
}
