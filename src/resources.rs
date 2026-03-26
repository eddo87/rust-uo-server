/// Resource gathering system for Ultima Online.
///
/// Supports three gathering skills (Mining, Lumberjacking, Fishing) each with
/// tiered resource types unlocked by player skill level. Resource nodes exist
/// in the world with finite amounts that respawn over time.

// ---------------------------------------------------------------------------
// Gathering skills
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatheringSkill {
    Mining,
    Lumberjacking,
    Fishing,
}

// ---------------------------------------------------------------------------
// Ore types – 9 tiers, Iron through Valorite
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OreType {
    Iron,
    DullCopper,
    ShadowIron,
    Copper,
    Bronze,
    Gold,
    Agapite,
    Verite,
    Valorite,
}

impl OreType {
    /// Minimum mining skill required to have a chance at this ore.
    pub fn min_skill(self) -> f64 {
        match self {
            OreType::Iron => 0.0,
            OreType::DullCopper => 25.0,
            OreType::ShadowIron => 35.0,
            OreType::Copper => 45.0,
            OreType::Bronze => 55.0,
            OreType::Gold => 65.0,
            OreType::Agapite => 75.0,
            OreType::Verite => 85.0,
            OreType::Valorite => 95.0,
        }
    }

    /// All ore types ordered from lowest to highest tier.
    pub fn all() -> &'static [OreType] {
        &[
            OreType::Iron,
            OreType::DullCopper,
            OreType::ShadowIron,
            OreType::Copper,
            OreType::Bronze,
            OreType::Gold,
            OreType::Agapite,
            OreType::Verite,
            OreType::Valorite,
        ]
    }
}

// ---------------------------------------------------------------------------
// Wood types – 7 tiers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WoodType {
    RegularWood,
    Oak,
    Ash,
    Yew,
    Heartwood,
    Bloodwood,
    Frostwood,
}

impl WoodType {
    pub fn min_skill(self) -> f64 {
        match self {
            WoodType::RegularWood => 0.0,
            WoodType::Oak => 30.0,
            WoodType::Ash => 45.0,
            WoodType::Yew => 55.0,
            WoodType::Heartwood => 65.0,
            WoodType::Bloodwood => 80.0,
            WoodType::Frostwood => 95.0,
        }
    }

    pub fn all() -> &'static [WoodType] {
        &[
            WoodType::RegularWood,
            WoodType::Oak,
            WoodType::Ash,
            WoodType::Yew,
            WoodType::Heartwood,
            WoodType::Bloodwood,
            WoodType::Frostwood,
        ]
    }
}

// ---------------------------------------------------------------------------
// Fish types – 5 types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FishType {
    Fish,
    Trout,
    Salmon,
    RedSnapper,
    BigFish,
}

impl FishType {
    pub fn min_skill(self) -> f64 {
        match self {
            FishType::Fish => 0.0,
            FishType::Trout => 25.0,
            FishType::Salmon => 50.0,
            FishType::RedSnapper => 75.0,
            FishType::BigFish => 90.0,
        }
    }

    pub fn all() -> &'static [FishType] {
        &[
            FishType::Fish,
            FishType::Trout,
            FishType::Salmon,
            FishType::RedSnapper,
            FishType::BigFish,
        ]
    }
}

// ---------------------------------------------------------------------------
// Resource kind – wraps the three type enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Ore(OreType),
    Wood(WoodType),
    Fish(FishType),
}

// ---------------------------------------------------------------------------
// Gather result
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum GatherResult {
    /// Successful gather: resource kind and amount obtained.
    Success { kind: ResourceKind, amount: u32 },
    /// The player's skill is too low for any resource at this node.
    SkillTooLow,
    /// The resource node is depleted and must respawn.
    Depleted,
    /// No resource node found near the given position.
    NoResource,
}

// ---------------------------------------------------------------------------
// Position helper (simple 2-D world coordinate)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Squared Euclidean distance (avoids sqrt for radius checks).
    pub fn distance_sq(self, other: Position) -> i64 {
        let dx = (self.x as i64) - (other.x as i64);
        let dy = (self.y as i64) - (other.y as i64);
        dx * dx + dy * dy
    }
}

// ---------------------------------------------------------------------------
// Resource node
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ResourceNode {
    pub position: Position,
    pub kind: ResourceKind,
    /// Current amount of resource left in this node.
    pub remaining: u32,
    /// Maximum amount the node can hold (used for respawning).
    pub max_amount: u32,
    /// Respawn timer: ticks remaining until the node is fully replenished.
    /// Zero means the node is available (may still have partial resources).
    pub respawn_timer: u64,
    /// How many ticks a full respawn takes.
    pub respawn_duration: u64,
}

impl ResourceNode {
    pub fn new(
        position: Position,
        kind: ResourceKind,
        max_amount: u32,
        respawn_duration: u64,
    ) -> Self {
        Self {
            position,
            kind,
            remaining: max_amount,
            max_amount,
            respawn_timer: 0,
            respawn_duration,
        }
    }

    /// Returns `true` when the node has been fully depleted.
    pub fn is_depleted(&self) -> bool {
        self.remaining == 0
    }
}

// ---------------------------------------------------------------------------
// Skill-check helpers
// ---------------------------------------------------------------------------

/// Returns the best ore type a player can mine at the given skill level.
pub fn check_ore_type(skill: f64) -> Option<OreType> {
    OreType::all()
        .iter()
        .rev()
        .find(|ore| skill >= ore.min_skill())
        .copied()
}

/// Returns the best wood type a player can chop at the given skill level.
pub fn check_wood_type(skill: f64) -> Option<WoodType> {
    WoodType::all()
        .iter()
        .rev()
        .find(|wood| skill >= wood.min_skill())
        .copied()
}

/// Returns the best fish type a player can catch at the given skill level.
pub fn check_fish_type(skill: f64) -> Option<FishType> {
    FishType::all()
        .iter()
        .rev()
        .find(|fish| skill >= fish.min_skill())
        .copied()
}

// ---------------------------------------------------------------------------
// Resource manager
// ---------------------------------------------------------------------------

pub struct ResourceManager {
    pub nodes: Vec<ResourceNode>,
}

impl ResourceManager {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Register a new resource node in the world.
    pub fn add_node(&mut self, node: ResourceNode) {
        self.nodes.push(node);
    }

    /// Return references to every node within `radius` tiles of `pos`.
    pub fn get_nodes_near(&self, pos: Position, radius: i32) -> Vec<&ResourceNode> {
        let radius_sq = (radius as i64) * (radius as i64);
        self.nodes
            .iter()
            .filter(|n| n.position.distance_sq(pos) <= radius_sq)
            .collect()
    }

    /// Attempt to gather from the nearest suitable node within `radius` of
    /// `pos`. The `skill` value corresponds to the player's level in the
    /// gathering discipline that matches the node's resource kind.
    ///
    /// On success one unit is removed from the node and a `GatherResult::Success`
    /// is returned. When a node is fully depleted its respawn timer starts.
    pub fn gather_at(
        &mut self,
        pos: Position,
        skill: f64,
        desired_skill: GatheringSkill,
        radius: i32,
    ) -> GatherResult {
        let radius_sq = (radius as i64) * (radius as i64);

        // Find the closest matching node.
        let maybe_idx = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| {
                n.position.distance_sq(pos) <= radius_sq && skill_matches(desired_skill, n.kind)
            })
            .min_by_key(|(_, n)| n.position.distance_sq(pos))
            .map(|(i, _)| i);

        let idx = match maybe_idx {
            Some(i) => i,
            None => return GatherResult::NoResource,
        };

        let node = &mut self.nodes[idx];

        if node.is_depleted() {
            return GatherResult::Depleted;
        }

        // Determine the best resource tier the player can obtain.
        let best_kind = match node.kind {
            ResourceKind::Ore(_) => check_ore_type(skill).map(ResourceKind::Ore),
            ResourceKind::Wood(_) => check_wood_type(skill).map(ResourceKind::Wood),
            ResourceKind::Fish(_) => check_fish_type(skill).map(ResourceKind::Fish),
        };

        let kind = match best_kind {
            Some(k) => k,
            None => return GatherResult::SkillTooLow,
        };

        // Deduct one unit.
        node.remaining -= 1;

        // If now depleted, start the respawn timer.
        if node.is_depleted() {
            node.respawn_timer = node.respawn_duration;
        }

        GatherResult::Success { kind, amount: 1 }
    }

    /// Advance all respawn timers by `elapsed` ticks. Nodes whose timer
    /// reaches zero are fully replenished.
    pub fn update_respawns(&mut self, elapsed: u64) {
        for node in &mut self.nodes {
            if node.respawn_timer > 0 {
                if node.respawn_timer <= elapsed {
                    node.respawn_timer = 0;
                    node.remaining = node.max_amount;
                } else {
                    node.respawn_timer -= elapsed;
                }
            }
        }
    }
}

/// Returns `true` when the gathering skill is appropriate for the resource kind.
fn skill_matches(skill: GatheringSkill, kind: ResourceKind) -> bool {
    matches!(
        (skill, kind),
        (GatheringSkill::Mining, ResourceKind::Ore(_))
            | (GatheringSkill::Lumberjacking, ResourceKind::Wood(_))
            | (GatheringSkill::Fishing, ResourceKind::Fish(_))
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Ore skill checks --------------------------------------------------

    #[test]
    fn check_ore_type_zero_skill_returns_iron() {
        assert_eq!(check_ore_type(0.0), Some(OreType::Iron));
    }

    #[test]
    fn check_ore_type_mid_skill_returns_bronze() {
        assert_eq!(check_ore_type(55.0), Some(OreType::Bronze));
    }

    #[test]
    fn check_ore_type_max_skill_returns_valorite() {
        assert_eq!(check_ore_type(100.0), Some(OreType::Valorite));
    }

    #[test]
    fn check_ore_type_between_tiers() {
        // 60.0 is above Bronze (55) but below Gold (65)
        assert_eq!(check_ore_type(60.0), Some(OreType::Bronze));
    }

    // -- Wood skill checks -------------------------------------------------

    #[test]
    fn check_wood_type_zero_skill_returns_regular() {
        assert_eq!(check_wood_type(0.0), Some(WoodType::RegularWood));
    }

    #[test]
    fn check_wood_type_high_skill_returns_frostwood() {
        assert_eq!(check_wood_type(95.0), Some(WoodType::Frostwood));
    }

    #[test]
    fn check_wood_type_mid_skill() {
        assert_eq!(check_wood_type(50.0), Some(WoodType::Ash));
    }

    // -- Fish skill checks -------------------------------------------------

    #[test]
    fn check_fish_type_zero_skill_returns_fish() {
        assert_eq!(check_fish_type(0.0), Some(FishType::Fish));
    }

    #[test]
    fn check_fish_type_high_skill_returns_bigfish() {
        assert_eq!(check_fish_type(90.0), Some(FishType::BigFish));
    }

    #[test]
    fn check_fish_type_mid_skill() {
        // 60.0 is above Salmon (50) but below RedSnapper (75)
        assert_eq!(check_fish_type(60.0), Some(FishType::Salmon));
    }

    // -- Position ----------------------------------------------------------

    #[test]
    fn position_distance_sq_basic() {
        let a = Position::new(0, 0);
        let b = Position::new(3, 4);
        assert_eq!(a.distance_sq(b), 25);
    }

    // -- ResourceNode ------------------------------------------------------

    #[test]
    fn resource_node_depleted_when_zero_remaining() {
        let mut node = ResourceNode::new(
            Position::new(10, 10),
            ResourceKind::Ore(OreType::Iron),
            5,
            100,
        );
        assert!(!node.is_depleted());
        node.remaining = 0;
        assert!(node.is_depleted());
    }

    // -- ResourceManager: add & query --------------------------------------

    #[test]
    fn add_node_and_get_nodes_near() {
        let mut mgr = ResourceManager::new();
        mgr.add_node(ResourceNode::new(
            Position::new(100, 100),
            ResourceKind::Ore(OreType::Iron),
            10,
            200,
        ));
        mgr.add_node(ResourceNode::new(
            Position::new(500, 500),
            ResourceKind::Wood(WoodType::Oak),
            8,
            300,
        ));

        let near = mgr.get_nodes_near(Position::new(100, 102), 5);
        assert_eq!(near.len(), 1);
        assert_eq!(near[0].position, Position::new(100, 100));
    }

    #[test]
    fn get_nodes_near_returns_empty_when_none_in_range() {
        let mut mgr = ResourceManager::new();
        mgr.add_node(ResourceNode::new(
            Position::new(100, 100),
            ResourceKind::Ore(OreType::Iron),
            10,
            200,
        ));
        let near = mgr.get_nodes_near(Position::new(0, 0), 5);
        assert!(near.is_empty());
    }

    // -- ResourceManager: gather -------------------------------------------

    #[test]
    fn gather_success_removes_one_unit() {
        let mut mgr = ResourceManager::new();
        mgr.add_node(ResourceNode::new(
            Position::new(10, 10),
            ResourceKind::Ore(OreType::Iron),
            3,
            100,
        ));

        let result = mgr.gather_at(Position::new(10, 10), 50.0, GatheringSkill::Mining, 2);
        assert!(matches!(result, GatherResult::Success { amount: 1, .. }));
        assert_eq!(mgr.nodes[0].remaining, 2);
    }

    #[test]
    fn gather_returns_best_ore_for_skill() {
        let mut mgr = ResourceManager::new();
        mgr.add_node(ResourceNode::new(
            Position::new(0, 0),
            ResourceKind::Ore(OreType::Valorite),
            5,
            100,
        ));

        let result = mgr.gather_at(Position::new(0, 0), 55.0, GatheringSkill::Mining, 2);
        // Skill 55 -> Bronze
        assert_eq!(
            result,
            GatherResult::Success {
                kind: ResourceKind::Ore(OreType::Bronze),
                amount: 1
            }
        );
    }

    #[test]
    fn gather_depleted_starts_respawn_timer() {
        let mut mgr = ResourceManager::new();
        mgr.add_node(ResourceNode::new(
            Position::new(5, 5),
            ResourceKind::Ore(OreType::Iron),
            1,
            200,
        ));

        let _ = mgr.gather_at(Position::new(5, 5), 10.0, GatheringSkill::Mining, 2);
        assert!(mgr.nodes[0].is_depleted());
        assert_eq!(mgr.nodes[0].respawn_timer, 200);
    }

    #[test]
    fn gather_from_depleted_node_returns_depleted() {
        let mut mgr = ResourceManager::new();
        mgr.add_node(ResourceNode::new(
            Position::new(0, 0),
            ResourceKind::Wood(WoodType::RegularWood),
            1,
            100,
        ));

        // First gather depletes.
        let _ = mgr.gather_at(Position::new(0, 0), 10.0, GatheringSkill::Lumberjacking, 2);
        // Second should report depleted.
        let result = mgr.gather_at(Position::new(0, 0), 10.0, GatheringSkill::Lumberjacking, 2);
        assert_eq!(result, GatherResult::Depleted);
    }

    #[test]
    fn gather_no_resource_when_wrong_skill() {
        let mut mgr = ResourceManager::new();
        mgr.add_node(ResourceNode::new(
            Position::new(0, 0),
            ResourceKind::Ore(OreType::Iron),
            5,
            100,
        ));

        let result = mgr.gather_at(Position::new(0, 0), 50.0, GatheringSkill::Fishing, 2);
        assert_eq!(result, GatherResult::NoResource);
    }

    #[test]
    fn gather_no_resource_when_out_of_range() {
        let mut mgr = ResourceManager::new();
        mgr.add_node(ResourceNode::new(
            Position::new(100, 100),
            ResourceKind::Fish(FishType::Fish),
            5,
            50,
        ));

        let result = mgr.gather_at(Position::new(0, 0), 10.0, GatheringSkill::Fishing, 5);
        assert_eq!(result, GatherResult::NoResource);
    }

    // -- ResourceManager: respawn ------------------------------------------

    #[test]
    fn update_respawns_replenishes_depleted_node() {
        let mut mgr = ResourceManager::new();
        mgr.add_node(ResourceNode::new(
            Position::new(0, 0),
            ResourceKind::Ore(OreType::Iron),
            1,
            100,
        ));

        // Deplete it.
        let _ = mgr.gather_at(Position::new(0, 0), 10.0, GatheringSkill::Mining, 2);
        assert!(mgr.nodes[0].is_depleted());

        // Partial tick — not yet replenished.
        mgr.update_respawns(50);
        assert!(mgr.nodes[0].is_depleted());
        assert_eq!(mgr.nodes[0].respawn_timer, 50);

        // Remaining ticks — should replenish.
        mgr.update_respawns(50);
        assert!(!mgr.nodes[0].is_depleted());
        assert_eq!(mgr.nodes[0].remaining, 1);
        assert_eq!(mgr.nodes[0].respawn_timer, 0);
    }

    #[test]
    fn update_respawns_overshoot_still_replenishes() {
        let mut mgr = ResourceManager::new();
        mgr.add_node(ResourceNode::new(
            Position::new(0, 0),
            ResourceKind::Wood(WoodType::RegularWood),
            3,
            10,
        ));

        // Deplete.
        for _ in 0..3 {
            let _ = mgr.gather_at(Position::new(0, 0), 10.0, GatheringSkill::Lumberjacking, 2);
        }
        assert!(mgr.nodes[0].is_depleted());

        // Overshoot the timer.
        mgr.update_respawns(999);
        assert_eq!(mgr.nodes[0].remaining, 3);
        assert_eq!(mgr.nodes[0].respawn_timer, 0);
    }

    #[test]
    fn update_respawns_ignores_non_depleted_nodes() {
        let mut mgr = ResourceManager::new();
        mgr.add_node(ResourceNode::new(
            Position::new(0, 0),
            ResourceKind::Fish(FishType::Salmon),
            5,
            100,
        ));

        // Node is full — timer is already 0.
        mgr.update_respawns(50);
        assert_eq!(mgr.nodes[0].remaining, 5);
        assert_eq!(mgr.nodes[0].respawn_timer, 0);
    }

    // -- Min-skill sanity checks -------------------------------------------

    #[test]
    fn ore_min_skills_are_ascending() {
        let ores = OreType::all();
        for pair in ores.windows(2) {
            assert!(
                pair[0].min_skill() <= pair[1].min_skill(),
                "{:?} min_skill should be <= {:?} min_skill",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn wood_min_skills_are_ascending() {
        let woods = WoodType::all();
        for pair in woods.windows(2) {
            assert!(
                pair[0].min_skill() <= pair[1].min_skill(),
                "{:?} min_skill should be <= {:?} min_skill",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn fish_min_skills_are_ascending() {
        let fishes = FishType::all();
        for pair in fishes.windows(2) {
            assert!(
                pair[0].min_skill() <= pair[1].min_skill(),
                "{:?} min_skill should be <= {:?} min_skill",
                pair[0],
                pair[1]
            );
        }
    }

    // -- Fishing gather round-trip -----------------------------------------

    #[test]
    fn gather_fish_success() {
        let mut mgr = ResourceManager::new();
        mgr.add_node(ResourceNode::new(
            Position::new(50, 50),
            ResourceKind::Fish(FishType::BigFish),
            2,
            60,
        ));

        let result = mgr.gather_at(Position::new(50, 51), 90.0, GatheringSkill::Fishing, 3);
        assert_eq!(
            result,
            GatherResult::Success {
                kind: ResourceKind::Fish(FishType::BigFish),
                amount: 1
            }
        );
    }
}
