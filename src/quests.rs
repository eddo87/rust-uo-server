use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuestType {
    Kill,
    Gather,
    Deliver,
    Escort,
    Explore,
    Craft,
    Skill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuestDifficulty {
    Trivial,
    Easy,
    Medium,
    Hard,
    Elite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestStatus {
    NotStarted,
    InProgress,
    Completed,
    Failed,
    TurnedIn,
}

// ---------------------------------------------------------------------------
// Quest building blocks
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct QuestObjective {
    pub description: String,
    pub quest_type: QuestType,
    pub target: String,
    pub required_count: u32,
    pub current_count: u32,
}

impl QuestObjective {
    pub fn new(description: &str, quest_type: QuestType, target: &str, required_count: u32) -> Self {
        Self {
            description: description.to_string(),
            quest_type,
            target: target.to_string(),
            required_count,
            current_count: 0,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.current_count >= self.required_count
    }

    pub fn advance(&mut self, amount: u32) {
        self.current_count = (self.current_count + amount).min(self.required_count);
    }
}

#[derive(Debug, Clone)]
pub struct SkillGain {
    pub skill_name: String,
    pub amount: f32,
}

#[derive(Debug, Clone)]
pub struct QuestReward {
    pub gold: u32,
    pub items: Vec<String>,
    pub skill_gains: Vec<SkillGain>,
    pub fame: i32,
    pub karma: i32,
}

impl QuestReward {
    pub fn new() -> Self {
        Self {
            gold: 0,
            items: Vec::new(),
            skill_gains: Vec::new(),
            fame: 0,
            karma: 0,
        }
    }
}

impl Default for QuestReward {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Quest definition (static template)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct QuestDefinition {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub difficulty: QuestDifficulty,
    pub objectives: Vec<QuestObjective>,
    pub rewards: QuestReward,
    pub prerequisites: Vec<u32>,
    pub repeatable: bool,
    /// Time limit in seconds; `None` means no limit.
    pub time_limit: Option<u64>,
    /// Id of the next quest in a chain; `None` if standalone.
    pub chain_quest: Option<u32>,
}

impl QuestDefinition {
    pub fn new(id: u32, name: &str, description: &str, difficulty: QuestDifficulty) -> Self {
        Self {
            id,
            name: name.to_string(),
            description: description.to_string(),
            difficulty,
            objectives: Vec::new(),
            rewards: QuestReward::new(),
            prerequisites: Vec::new(),
            repeatable: false,
            time_limit: None,
            chain_quest: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Active quest (live instance owned by a player)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ActiveQuest {
    pub quest_id: u32,
    pub status: QuestStatus,
    pub objectives: Vec<QuestObjective>,
    /// Tick (epoch millis) when the quest was accepted.
    pub started_at: i64,
    /// Optional deadline tick; `None` if no time limit.
    pub deadline: Option<i64>,
}

impl ActiveQuest {
    /// Create a new active quest from a definition, stamping the current time.
    pub fn from_definition(def: &QuestDefinition, current_ticks: i64) -> Self {
        let deadline = def.time_limit.map(|secs| current_ticks + (secs as i64) * 1000);
        Self {
            quest_id: def.id,
            status: QuestStatus::InProgress,
            objectives: def.objectives.clone(),
            started_at: current_ticks,
            deadline,
        }
    }

    /// Advance a specific objective identified by its index.
    /// Returns `true` if all objectives are now complete.
    pub fn advance_objective(&mut self, index: usize, amount: u32) -> bool {
        if self.status != QuestStatus::InProgress {
            return false;
        }
        if let Some(obj) = self.objectives.get_mut(index) {
            obj.advance(amount);
        }
        if self.all_objectives_complete() {
            self.status = QuestStatus::Completed;
            true
        } else {
            false
        }
    }

    /// Check whether every objective has met its required count.
    pub fn all_objectives_complete(&self) -> bool {
        self.objectives.iter().all(|o| o.is_complete())
    }

    /// Mark the quest as turned in (rewards granted externally).
    pub fn turn_in(&mut self) -> bool {
        if self.status == QuestStatus::Completed {
            self.status = QuestStatus::TurnedIn;
            true
        } else {
            false
        }
    }

    /// Fail the quest (e.g. time expired).
    pub fn fail(&mut self) {
        if self.status == QuestStatus::InProgress {
            self.status = QuestStatus::Failed;
        }
    }

    /// Check the deadline against the current time and fail if expired.
    pub fn check_deadline(&mut self, current_ticks: i64) {
        if let Some(deadline) = self.deadline {
            if current_ticks >= deadline && self.status == QuestStatus::InProgress {
                self.fail();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Per-player quest log
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct QuestLog {
    pub active_quests: Vec<ActiveQuest>,
    pub completed_quest_ids: Vec<u32>,
    pub max_active_quests: usize,
}

impl QuestLog {
    pub fn new(max_active_quests: usize) -> Self {
        Self {
            active_quests: Vec::new(),
            completed_quest_ids: Vec::new(),
            max_active_quests,
        }
    }

    /// Accept a quest from the registry, checking prerequisites and capacity.
    pub fn accept_quest(
        &mut self,
        registry: &QuestRegistry,
        quest_id: u32,
        current_ticks: i64,
    ) -> Result<(), String> {
        let def = registry
            .get(quest_id)
            .ok_or_else(|| format!("Quest {} not found in registry", quest_id))?;

        // Capacity check
        if self.active_quests.len() >= self.max_active_quests {
            return Err("Quest log is full".to_string());
        }

        // Already active?
        if self.active_quests.iter().any(|q| q.quest_id == quest_id) {
            return Err(format!("Quest {} is already active", quest_id));
        }

        // Already completed and not repeatable?
        if self.completed_quest_ids.contains(&quest_id) && !def.repeatable {
            return Err(format!("Quest {} has already been completed", quest_id));
        }

        // Prerequisites
        for &pre in &def.prerequisites {
            if !self.completed_quest_ids.contains(&pre) {
                return Err(format!("Prerequisite quest {} not completed", pre));
            }
        }

        let active = ActiveQuest::from_definition(def, current_ticks);
        self.active_quests.push(active);
        Ok(())
    }

    /// Turn in a completed quest, recording it in history.
    pub fn turn_in_quest(&mut self, quest_id: u32) -> Result<(), String> {
        let quest = self
            .active_quests
            .iter_mut()
            .find(|q| q.quest_id == quest_id)
            .ok_or_else(|| format!("Quest {} is not active", quest_id))?;

        if !quest.turn_in() {
            return Err(format!("Quest {} is not completed yet", quest_id));
        }

        self.completed_quest_ids.push(quest_id);
        self.active_quests.retain(|q| q.quest_id != quest_id);
        Ok(())
    }

    /// Abandon an active quest.
    pub fn abandon_quest(&mut self, quest_id: u32) -> Result<(), String> {
        let len_before = self.active_quests.len();
        self.active_quests.retain(|q| q.quest_id != quest_id);
        if self.active_quests.len() == len_before {
            Err(format!("Quest {} is not active", quest_id))
        } else {
            Ok(())
        }
    }

    /// Tick all active quests to check deadlines.
    pub fn check_deadlines(&mut self, current_ticks: i64) {
        for quest in &mut self.active_quests {
            quest.check_deadline(current_ticks);
        }
    }
}

// ---------------------------------------------------------------------------
// Global quest registry
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct QuestRegistry {
    quests: HashMap<u32, QuestDefinition>,
}

impl QuestRegistry {
    pub fn new() -> Self {
        Self {
            quests: HashMap::new(),
        }
    }

    pub fn register(&mut self, def: QuestDefinition) {
        self.quests.insert(def.id, def);
    }

    pub fn get(&self, id: u32) -> Option<&QuestDefinition> {
        self.quests.get(&id)
    }

    pub fn all(&self) -> Vec<&QuestDefinition> {
        self.quests.values().collect()
    }

    pub fn count(&self) -> usize {
        self.quests.len()
    }
}

impl Default for QuestRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Pre-registered example quests
// ---------------------------------------------------------------------------

/// Build a registry pre-populated with four example quests.
pub fn create_default_registry() -> QuestRegistry {
    let mut registry = QuestRegistry::new();

    // 1 -- Kill quest: slay skeletons in the graveyard
    let mut q1 = QuestDefinition::new(
        1,
        "Cleansing the Graveyard",
        "The cemetery outside Britain is overrun with undead. Destroy the skeletons!",
        QuestDifficulty::Easy,
    );
    q1.objectives.push(QuestObjective::new(
        "Slay skeletons",
        QuestType::Kill,
        "skeleton",
        10,
    ));
    q1.rewards = QuestReward {
        gold: 500,
        items: vec!["iron_shield".to_string()],
        skill_gains: vec![SkillGain {
            skill_name: "swordsmanship".to_string(),
            amount: 1.5,
        }],
        fame: 50,
        karma: 25,
    };
    q1.repeatable = true;
    registry.register(q1);

    // 2 -- Gather quest: collect reagents
    let mut q2 = QuestDefinition::new(
        2,
        "Reagent Resupply",
        "The mage tower in Moonglow is running low on black pearl. Gather some from the coast.",
        QuestDifficulty::Trivial,
    );
    q2.objectives.push(QuestObjective::new(
        "Collect black pearls",
        QuestType::Gather,
        "black_pearl",
        20,
    ));
    q2.rewards = QuestReward {
        gold: 300,
        items: vec![],
        skill_gains: vec![SkillGain {
            skill_name: "magery".to_string(),
            amount: 0.5,
        }],
        fame: 10,
        karma: 10,
    };
    q2.repeatable = true;
    registry.register(q2);

    // 3 -- Escort quest: guard an NPC traveller (chained to quest 1)
    let mut q3 = QuestDefinition::new(
        3,
        "Safe Passage to Trinsic",
        "A merchant needs an armed escort from Britain to Trinsic. Keep them alive!",
        QuestDifficulty::Medium,
    );
    q3.objectives.push(QuestObjective::new(
        "Escort the merchant to Trinsic",
        QuestType::Escort,
        "merchant_npc",
        1,
    ));
    q3.rewards = QuestReward {
        gold: 1000,
        items: vec!["magic_map".to_string()],
        skill_gains: vec![],
        fame: 100,
        karma: 50,
    };
    q3.prerequisites.push(1);
    q3.time_limit = Some(3600); // one hour
    registry.register(q3);

    // 4 -- Craft quest: forge horseshoes (part of a chain)
    let mut q4 = QuestDefinition::new(
        4,
        "The Blacksmith's Challenge",
        "Prove your skill at the forge by crafting horseshoes for the Britain stables.",
        QuestDifficulty::Hard,
    );
    q4.objectives.push(QuestObjective::new(
        "Craft horseshoes",
        QuestType::Craft,
        "horseshoe",
        5,
    ));
    q4.objectives.push(QuestObjective::new(
        "Deliver horseshoes to the stable master",
        QuestType::Deliver,
        "stable_master",
        1,
    ));
    q4.rewards = QuestReward {
        gold: 750,
        items: vec!["smithing_hammer_+1".to_string()],
        skill_gains: vec![SkillGain {
            skill_name: "blacksmithing".to_string(),
            amount: 2.0,
        }],
        fame: 75,
        karma: 0,
    };
    q4.chain_quest = Some(5); // hypothetical follow-up
    registry.register(q4);

    registry
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> QuestRegistry {
        create_default_registry()
    }

    // --- Registry tests ---

    #[test]
    fn test_default_registry_has_four_quests() {
        let reg = setup();
        assert_eq!(reg.count(), 4);
    }

    #[test]
    fn test_registry_get_existing() {
        let reg = setup();
        let q = reg.get(1).unwrap();
        assert_eq!(q.name, "Cleansing the Graveyard");
    }

    #[test]
    fn test_registry_get_missing() {
        let reg = setup();
        assert!(reg.get(999).is_none());
    }

    #[test]
    fn test_registry_all() {
        let reg = setup();
        let all = reg.all();
        assert_eq!(all.len(), 4);
    }

    // --- QuestObjective tests ---

    #[test]
    fn test_objective_advance_and_complete() {
        let mut obj = QuestObjective::new("Kill rats", QuestType::Kill, "rat", 5);
        assert!(!obj.is_complete());

        obj.advance(3);
        assert_eq!(obj.current_count, 3);
        assert!(!obj.is_complete());

        obj.advance(5); // clamped to required_count
        assert_eq!(obj.current_count, 5);
        assert!(obj.is_complete());
    }

    #[test]
    fn test_objective_advance_clamped() {
        let mut obj = QuestObjective::new("Gather ore", QuestType::Gather, "ore", 3);
        obj.advance(100);
        assert_eq!(obj.current_count, 3);
    }

    // --- ActiveQuest tests ---

    #[test]
    fn test_active_quest_from_definition() {
        let reg = setup();
        let def = reg.get(1).unwrap();
        let aq = ActiveQuest::from_definition(def, 1000);
        assert_eq!(aq.status, QuestStatus::InProgress);
        assert_eq!(aq.objectives.len(), 1);
        assert_eq!(aq.started_at, 1000);
        assert!(aq.deadline.is_none()); // quest 1 has no time limit
    }

    #[test]
    fn test_active_quest_with_deadline() {
        let reg = setup();
        let def = reg.get(3).unwrap(); // escort quest, 3600s limit
        let aq = ActiveQuest::from_definition(def, 5000);
        assert_eq!(aq.deadline, Some(5000 + 3_600_000));
    }

    #[test]
    fn test_advance_objective_completes_quest() {
        let reg = setup();
        let def = reg.get(1).unwrap();
        let mut aq = ActiveQuest::from_definition(def, 0);
        assert!(!aq.advance_objective(0, 5));
        assert_eq!(aq.status, QuestStatus::InProgress);
        assert!(aq.advance_objective(0, 5));
        assert_eq!(aq.status, QuestStatus::Completed);
    }

    #[test]
    fn test_advance_does_nothing_when_not_in_progress() {
        let reg = setup();
        let def = reg.get(1).unwrap();
        let mut aq = ActiveQuest::from_definition(def, 0);
        aq.fail();
        let result = aq.advance_objective(0, 10);
        assert!(!result);
        assert_eq!(aq.objectives[0].current_count, 0);
    }

    #[test]
    fn test_turn_in_success() {
        let reg = setup();
        let def = reg.get(1).unwrap();
        let mut aq = ActiveQuest::from_definition(def, 0);
        aq.advance_objective(0, 10);
        assert!(aq.turn_in());
        assert_eq!(aq.status, QuestStatus::TurnedIn);
    }

    #[test]
    fn test_turn_in_fails_when_not_completed() {
        let reg = setup();
        let def = reg.get(1).unwrap();
        let mut aq = ActiveQuest::from_definition(def, 0);
        assert!(!aq.turn_in());
    }

    #[test]
    fn test_deadline_check_fails_quest() {
        let reg = setup();
        let def = reg.get(3).unwrap();
        let mut aq = ActiveQuest::from_definition(def, 0);
        // Not expired yet
        aq.check_deadline(1_000_000);
        assert_eq!(aq.status, QuestStatus::InProgress);
        // Expired
        aq.check_deadline(3_600_001);
        assert_eq!(aq.status, QuestStatus::Failed);
    }

    #[test]
    fn test_multi_objective_quest() {
        let reg = setup();
        let def = reg.get(4).unwrap(); // craft quest: 2 objectives
        let mut aq = ActiveQuest::from_definition(def, 0);
        assert_eq!(aq.objectives.len(), 2);

        // Complete first objective only
        aq.advance_objective(0, 5);
        assert_eq!(aq.status, QuestStatus::InProgress); // still in progress

        // Complete second objective
        assert!(aq.advance_objective(1, 1));
        assert_eq!(aq.status, QuestStatus::Completed);
    }

    // --- QuestLog tests ---

    #[test]
    fn test_accept_quest() {
        let reg = setup();
        let mut log = QuestLog::new(10);
        assert!(log.accept_quest(&reg, 1, 0).is_ok());
        assert_eq!(log.active_quests.len(), 1);
    }

    #[test]
    fn test_accept_quest_not_found() {
        let reg = setup();
        let mut log = QuestLog::new(10);
        let result = log.accept_quest(&reg, 999, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_accept_quest_full_log() {
        let reg = setup();
        let mut log = QuestLog::new(1);
        log.accept_quest(&reg, 1, 0).unwrap();
        let result = log.accept_quest(&reg, 2, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("full"));
    }

    #[test]
    fn test_accept_quest_already_active() {
        let reg = setup();
        let mut log = QuestLog::new(10);
        log.accept_quest(&reg, 1, 0).unwrap();
        let result = log.accept_quest(&reg, 1, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already active"));
    }

    #[test]
    fn test_accept_quest_prerequisite_not_met() {
        let reg = setup();
        let mut log = QuestLog::new(10);
        // Quest 3 requires quest 1 completed
        let result = log.accept_quest(&reg, 3, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Prerequisite"));
    }

    #[test]
    fn test_accept_quest_prerequisite_met() {
        let reg = setup();
        let mut log = QuestLog::new(10);
        // Complete quest 1 first
        log.accept_quest(&reg, 1, 0).unwrap();
        log.active_quests[0].advance_objective(0, 10);
        log.turn_in_quest(1).unwrap();
        // Now quest 3 should be available
        assert!(log.accept_quest(&reg, 3, 100).is_ok());
    }

    #[test]
    fn test_accept_non_repeatable_already_completed() {
        let reg = setup();
        let mut log = QuestLog::new(10);
        // Quest 3 is not repeatable; complete it first
        log.completed_quest_ids.push(1); // satisfy prerequisite
        log.accept_quest(&reg, 3, 0).unwrap();
        log.active_quests[0].advance_objective(0, 1);
        log.turn_in_quest(3).unwrap();
        // Try to accept again
        let result = log.accept_quest(&reg, 3, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already been completed"));
    }

    #[test]
    fn test_repeatable_quest_can_be_accepted_again() {
        let reg = setup();
        let mut log = QuestLog::new(10);
        // Quest 1 is repeatable
        log.accept_quest(&reg, 1, 0).unwrap();
        log.active_quests[0].advance_objective(0, 10);
        log.turn_in_quest(1).unwrap();
        assert!(log.accept_quest(&reg, 1, 100).is_ok());
    }

    #[test]
    fn test_turn_in_quest_via_log() {
        let reg = setup();
        let mut log = QuestLog::new(10);
        log.accept_quest(&reg, 1, 0).unwrap();
        log.active_quests[0].advance_objective(0, 10);
        assert!(log.turn_in_quest(1).is_ok());
        assert!(log.active_quests.is_empty());
        assert_eq!(log.completed_quest_ids, vec![1]);
    }

    #[test]
    fn test_turn_in_incomplete_quest() {
        let reg = setup();
        let mut log = QuestLog::new(10);
        log.accept_quest(&reg, 1, 0).unwrap();
        let result = log.turn_in_quest(1);
        assert!(result.is_err());
    }

    #[test]
    fn test_abandon_quest() {
        let reg = setup();
        let mut log = QuestLog::new(10);
        log.accept_quest(&reg, 1, 0).unwrap();
        assert!(log.abandon_quest(1).is_ok());
        assert!(log.active_quests.is_empty());
    }

    #[test]
    fn test_abandon_quest_not_active() {
        let mut log = QuestLog::new(10);
        let result = log.abandon_quest(1);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_deadlines() {
        let reg = setup();
        let mut log = QuestLog::new(10);
        log.completed_quest_ids.push(1); // prerequisite for quest 3
        log.accept_quest(&reg, 3, 0).unwrap();
        log.check_deadlines(3_600_001);
        assert_eq!(log.active_quests[0].status, QuestStatus::Failed);
    }

    // --- QuestDefinition tests ---

    #[test]
    fn test_quest_definition_chain() {
        let reg = setup();
        let q4 = reg.get(4).unwrap();
        assert_eq!(q4.chain_quest, Some(5));
    }

    #[test]
    fn test_quest_definition_no_chain() {
        let reg = setup();
        let q1 = reg.get(1).unwrap();
        assert!(q1.chain_quest.is_none());
    }

    // --- QuestReward default ---

    #[test]
    fn test_quest_reward_default() {
        let r = QuestReward::default();
        assert_eq!(r.gold, 0);
        assert!(r.items.is_empty());
        assert!(r.skill_gains.is_empty());
        assert_eq!(r.fame, 0);
        assert_eq!(r.karma, 0);
    }
}
