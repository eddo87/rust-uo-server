use rand::Rng;
use std::collections::HashMap;

/// Rarity tier for loot items in Ultima Online.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LootRarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

/// A single entry in a loot table describing one possible item drop.
#[derive(Debug, Clone)]
pub struct LootEntry {
    /// UO item ID (graphic / tile ID).
    pub item_id: u16,
    /// Human-readable item name.
    pub name: String,
    /// Minimum stack / quantity when this entry drops.
    pub min_amount: u16,
    /// Maximum stack / quantity when this entry drops.
    pub max_amount: u16,
    /// Probability that this entry drops, in the range `0.0..=1.0`.
    pub drop_chance: f64,
    /// Rarity classification.
    pub rarity: LootRarity,
    /// UO hue (colour) applied to the item. 0 = default hue.
    pub hue: u16,
}

/// Describes a gold drop with a random amount between `min` and `max`.
#[derive(Debug, Clone)]
pub struct GoldDrop {
    pub min_amount: u32,
    pub max_amount: u32,
}

/// A complete loot table: a gold range plus zero or more item entries.
#[derive(Debug, Clone)]
pub struct LootTable {
    pub name: String,
    pub gold: Option<GoldDrop>,
    pub entries: Vec<LootEntry>,
}

/// The result of rolling a single item from a loot table.
#[derive(Debug, Clone)]
pub struct LootDrop {
    pub item_id: u16,
    pub name: String,
    pub amount: u16,
    pub hue: u16,
    pub rarity: LootRarity,
}

/// Central registry that maps a table name to its `LootTable`.
#[derive(Debug)]
pub struct LootRegistry {
    tables: HashMap<String, LootTable>,
}

impl LootEntry {
    pub fn new(
        item_id: u16,
        name: &str,
        min_amount: u16,
        max_amount: u16,
        drop_chance: f64,
        rarity: LootRarity,
        hue: u16,
    ) -> Self {
        assert!(
            (0.0..=1.0).contains(&drop_chance),
            "drop_chance must be between 0.0 and 1.0"
        );
        assert!(
            min_amount <= max_amount,
            "min_amount must be <= max_amount"
        );
        Self {
            item_id,
            name: name.to_string(),
            min_amount,
            max_amount,
            drop_chance,
            rarity,
            hue,
        }
    }
}

impl GoldDrop {
    pub fn new(min_amount: u32, max_amount: u32) -> Self {
        assert!(min_amount <= max_amount, "min_amount must be <= max_amount");
        Self {
            min_amount,
            max_amount,
        }
    }

    /// Roll a random gold amount.
    pub fn roll(&self) -> u32 {
        if self.min_amount == self.max_amount {
            return self.min_amount;
        }
        let mut rng = rand::thread_rng();
        rng.gen_range(self.min_amount..=self.max_amount)
    }
}

impl LootTable {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            gold: None,
            entries: Vec::new(),
        }
    }

    pub fn with_gold(mut self, gold: GoldDrop) -> Self {
        self.gold = Some(gold);
        self
    }

    pub fn add_entry(mut self, entry: LootEntry) -> Self {
        self.entries.push(entry);
        self
    }

    /// Roll the table and return all drops (gold expressed as item_id 0x0EED).
    pub fn roll(&self) -> Vec<LootDrop> {
        let mut rng = rand::thread_rng();
        let mut drops = Vec::new();

        // Gold
        if let Some(ref gd) = self.gold {
            let amount = gd.roll();
            if amount > 0 {
                drops.push(LootDrop {
                    item_id: 0x0EED, // UO gold coin graphic
                    name: "Gold Coins".to_string(),
                    amount: amount.min(u16::MAX as u32) as u16,
                    hue: 0,
                    rarity: LootRarity::Common,
                });
            }
        }

        // Items
        for entry in &self.entries {
            let roll: f64 = rng.gen();
            if roll < entry.drop_chance {
                let amount = if entry.min_amount == entry.max_amount {
                    entry.min_amount
                } else {
                    rng.gen_range(entry.min_amount..=entry.max_amount)
                };
                drops.push(LootDrop {
                    item_id: entry.item_id,
                    name: entry.name.clone(),
                    amount,
                    hue: entry.hue,
                    rarity: entry.rarity,
                });
            }
        }

        drops
    }
}

impl LootRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
        }
    }

    /// Register a loot table under the given key.
    pub fn register(&mut self, key: &str, table: LootTable) {
        self.tables.insert(key.to_string(), table);
    }

    /// Look up a table by key.
    pub fn get(&self, key: &str) -> Option<&LootTable> {
        self.tables.get(key)
    }

    /// Roll a table by key, returning `None` if the key is not found.
    pub fn roll(&self, key: &str) -> Option<Vec<LootDrop>> {
        self.tables.get(key).map(|t| t.roll())
    }

    /// Number of registered tables.
    pub fn len(&self) -> usize {
        self.tables.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.tables.is_empty()
    }

    /// Create a registry pre-populated with default UO loot tables.
    pub fn with_defaults() -> Self {
        let mut reg = Self::new();

        // ── Skeleton ────────────────────────────────────────────────
        reg.register(
            "skeleton",
            LootTable::new("Skeleton")
                .with_gold(GoldDrop::new(25, 75))
                .add_entry(LootEntry::new(
                    0x0F5F, // bone
                    "Bone",
                    1, 3,
                    0.60,
                    LootRarity::Common,
                    0,
                ))
                .add_entry(LootEntry::new(
                    0x1B11, // bone armor (helm)
                    "Bone Helmet",
                    1, 1,
                    0.15,
                    LootRarity::Uncommon,
                    0,
                ))
                .add_entry(LootEntry::new(
                    0x0F52, // lesser heal potion
                    "Lesser Heal Potion",
                    1, 2,
                    0.20,
                    LootRarity::Common,
                    0,
                )),
        );

        // ── Dragon ──────────────────────────────────────────────────
        reg.register(
            "dragon",
            LootTable::new("Dragon")
                .with_gold(GoldDrop::new(800, 1600))
                .add_entry(LootEntry::new(
                    0x1F14, // dragon scales
                    "Dragon Scales",
                    5, 12,
                    0.80,
                    LootRarity::Uncommon,
                    0,
                ))
                .add_entry(LootEntry::new(
                    0x0F61, // greater heal potion
                    "Greater Heal Potion",
                    1, 3,
                    0.45,
                    LootRarity::Uncommon,
                    0,
                ))
                .add_entry(LootEntry::new(
                    0x13FF, // katana
                    "Fire Katana",
                    1, 1,
                    0.08,
                    LootRarity::Rare,
                    1161, // fire-red hue
                ))
                .add_entry(LootEntry::new(
                    0x2D01, // crystalline ring
                    "Crystalline Ring",
                    1, 1,
                    0.02,
                    LootRarity::Legendary,
                    1153, // bright white hue
                )),
        );

        // ── Orc ─────────────────────────────────────────────────────
        reg.register(
            "orc",
            LootTable::new("Orc")
                .with_gold(GoldDrop::new(50, 150))
                .add_entry(LootEntry::new(
                    0x13B9, // viking sword
                    "Orcish Sword",
                    1, 1,
                    0.25,
                    LootRarity::Common,
                    2207, // dirty-green hue
                ))
                .add_entry(LootEntry::new(
                    0x13BB, // chainmail coif
                    "Orc Helm",
                    1, 1,
                    0.20,
                    LootRarity::Common,
                    2207,
                ))
                .add_entry(LootEntry::new(
                    0x09D0, // raw ribs
                    "Raw Ribs",
                    1, 3,
                    0.50,
                    LootRarity::Common,
                    0,
                )),
        );

        // ── Merchant Crate ──────────────────────────────────────────
        reg.register(
            "merchant_crate",
            LootTable::new("Merchant Crate")
                .with_gold(GoldDrop::new(100, 400))
                .add_entry(LootEntry::new(
                    0x0F3F, // arrow
                    "Arrow",
                    10, 30,
                    0.55,
                    LootRarity::Common,
                    0,
                ))
                .add_entry(LootEntry::new(
                    0x0E34, // blank scroll
                    "Blank Scroll",
                    2, 8,
                    0.40,
                    LootRarity::Common,
                    0,
                ))
                .add_entry(LootEntry::new(
                    0x1F0D, // bolt of cloth
                    "Bolt of Cloth",
                    1, 4,
                    0.35,
                    LootRarity::Common,
                    0,
                ))
                .add_entry(LootEntry::new(
                    0x0F0E, // emerald
                    "Emerald",
                    1, 2,
                    0.10,
                    LootRarity::Rare,
                    0,
                )),
        );

        // ── Treasure Chest Level 1 ─────────────────────────────────
        reg.register(
            "treasure_chest_level1",
            LootTable::new("Treasure Chest (Level 1)")
                .with_gold(GoldDrop::new(150, 500))
                .add_entry(LootEntry::new(
                    0x0F09, // citrine
                    "Citrine",
                    1, 3,
                    0.30,
                    LootRarity::Uncommon,
                    0,
                ))
                .add_entry(LootEntry::new(
                    0x1086, // ring
                    "Silver Ring",
                    1, 1,
                    0.20,
                    LootRarity::Uncommon,
                    0,
                ))
                .add_entry(LootEntry::new(
                    0x0E21, // treasure map (level 2)
                    "Treasure Map (Level 2)",
                    1, 1,
                    0.05,
                    LootRarity::Rare,
                    0,
                ))
                .add_entry(LootEntry::new(
                    0x0F61, // greater heal potion
                    "Greater Heal Potion",
                    1, 3,
                    0.40,
                    LootRarity::Common,
                    0,
                )),
        );

        reg
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loot_rarity_equality() {
        assert_eq!(LootRarity::Common, LootRarity::Common);
        assert_ne!(LootRarity::Common, LootRarity::Legendary);
    }

    #[test]
    fn test_loot_entry_new() {
        let entry = LootEntry::new(0x0F5F, "Bone", 1, 3, 0.5, LootRarity::Common, 0);
        assert_eq!(entry.item_id, 0x0F5F);
        assert_eq!(entry.name, "Bone");
        assert_eq!(entry.min_amount, 1);
        assert_eq!(entry.max_amount, 3);
        assert!((entry.drop_chance - 0.5).abs() < f64::EPSILON);
        assert_eq!(entry.rarity, LootRarity::Common);
        assert_eq!(entry.hue, 0);
    }

    #[test]
    #[should_panic(expected = "drop_chance must be between 0.0 and 1.0")]
    fn test_loot_entry_invalid_drop_chance() {
        LootEntry::new(0x0001, "Bad", 1, 1, 1.5, LootRarity::Common, 0);
    }

    #[test]
    #[should_panic(expected = "min_amount must be <= max_amount")]
    fn test_loot_entry_invalid_amounts() {
        LootEntry::new(0x0001, "Bad", 5, 2, 0.5, LootRarity::Common, 0);
    }

    #[test]
    fn test_gold_drop_roll_range() {
        let gd = GoldDrop::new(10, 20);
        for _ in 0..100 {
            let amount = gd.roll();
            assert!(amount >= 10 && amount <= 20, "gold {} out of range", amount);
        }
    }

    #[test]
    fn test_gold_drop_fixed() {
        let gd = GoldDrop::new(42, 42);
        assert_eq!(gd.roll(), 42);
    }

    #[test]
    fn test_loot_table_guaranteed_drop() {
        let table = LootTable::new("test")
            .add_entry(LootEntry::new(
                0x0001, "Always", 1, 1, 1.0, LootRarity::Common, 0,
            ));
        // With drop_chance 1.0, the item should always appear.
        for _ in 0..50 {
            let drops = table.roll();
            assert!(
                drops.iter().any(|d| d.name == "Always"),
                "guaranteed entry missing"
            );
        }
    }

    #[test]
    fn test_loot_table_never_drop() {
        let table = LootTable::new("test")
            .add_entry(LootEntry::new(
                0x0001, "Never", 1, 1, 0.0, LootRarity::Common, 0,
            ));
        for _ in 0..50 {
            let drops = table.roll();
            assert!(
                !drops.iter().any(|d| d.name == "Never"),
                "zero-chance entry should never drop"
            );
        }
    }

    #[test]
    fn test_loot_table_with_gold() {
        let table = LootTable::new("test")
            .with_gold(GoldDrop::new(10, 100));
        let drops = table.roll();
        assert_eq!(drops.len(), 1);
        assert_eq!(drops[0].item_id, 0x0EED);
        assert_eq!(drops[0].name, "Gold Coins");
        assert!(drops[0].amount >= 10 && drops[0].amount <= 100);
    }

    #[test]
    fn test_loot_registry_basic() {
        let mut reg = LootRegistry::new();
        assert!(reg.is_empty());
        reg.register("test", LootTable::new("Test Table"));
        assert_eq!(reg.len(), 1);
        assert!(reg.get("test").is_some());
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn test_loot_registry_roll_missing_key() {
        let reg = LootRegistry::new();
        assert!(reg.roll("missing").is_none());
    }

    #[test]
    fn test_loot_registry_with_defaults() {
        let reg = LootRegistry::with_defaults();
        assert_eq!(reg.len(), 5);
        assert!(reg.get("skeleton").is_some());
        assert!(reg.get("dragon").is_some());
        assert!(reg.get("orc").is_some());
        assert!(reg.get("merchant_crate").is_some());
        assert!(reg.get("treasure_chest_level1").is_some());
    }

    #[test]
    fn test_default_skeleton_table_structure() {
        let reg = LootRegistry::with_defaults();
        let table = reg.get("skeleton").unwrap();
        assert_eq!(table.name, "Skeleton");
        assert!(table.gold.is_some());
        let gold = table.gold.as_ref().unwrap();
        assert_eq!(gold.min_amount, 25);
        assert_eq!(gold.max_amount, 75);
        assert_eq!(table.entries.len(), 3);
    }

    #[test]
    fn test_default_dragon_table_has_legendary() {
        let reg = LootRegistry::with_defaults();
        let table = reg.get("dragon").unwrap();
        assert!(
            table.entries.iter().any(|e| e.rarity == LootRarity::Legendary),
            "dragon table should contain at least one legendary entry"
        );
    }

    #[test]
    fn test_roll_default_tables_no_panic() {
        let reg = LootRegistry::with_defaults();
        for key in &["skeleton", "dragon", "orc", "merchant_crate", "treasure_chest_level1"] {
            let drops = reg.roll(key);
            assert!(drops.is_some(), "roll for '{}' should not be None", key);
        }
    }

    #[test]
    fn test_loot_drop_amount_within_bounds() {
        // Use a table where the item always drops with a range.
        let table = LootTable::new("test")
            .add_entry(LootEntry::new(
                0x0001, "Ranged", 3, 10, 1.0, LootRarity::Uncommon, 0,
            ));
        for _ in 0..100 {
            let drops = table.roll();
            let item = drops.iter().find(|d| d.name == "Ranged").unwrap();
            assert!(
                item.amount >= 3 && item.amount <= 10,
                "amount {} out of bounds",
                item.amount
            );
        }
    }
}
