/// NPC vendor/shop system for Ultima Online.
///
/// Handles shop inventories, pricing with markup/markdown, and buy/sell
/// transactions between players and NPC vendors.

// ---------------------------------------------------------------------------
// Core data types
// ---------------------------------------------------------------------------

/// A single item available in a vendor's shop.
#[derive(Debug, Clone, PartialEq)]
pub struct ShopItem {
    /// Internal item id (matches the game's item type id).
    pub item_id: u32,
    /// Human-readable name shown to the player.
    pub name: String,
    /// Base price in gold pieces before any vendor markup.
    pub base_price: u32,
    /// Current quantity the vendor has in stock.
    pub quantity: u32,
    /// Maximum quantity the vendor can carry for this item.
    pub max_quantity: u32,
    /// Weight per unit (in stones, times 10 for one decimal place).
    pub weight: u16,
}

/// The full inventory of a single vendor NPC.
#[derive(Debug, Clone)]
pub struct ShopInventory {
    pub items: Vec<ShopItem>,
}

impl ShopInventory {
    pub fn new() -> Self {
        ShopInventory { items: vec![] }
    }

    /// Add an item to the inventory. If the item_id already exists the
    /// quantities are merged instead of creating a duplicate entry.
    pub fn add_item(&mut self, item: ShopItem) {
        if let Some(existing) = self.items.iter_mut().find(|i| i.item_id == item.item_id) {
            existing.quantity = (existing.quantity + item.quantity).min(existing.max_quantity);
        } else {
            self.items.push(item);
        }
    }

    /// Look up an item by id.
    pub fn find_item(&self, item_id: u32) -> Option<&ShopItem> {
        self.items.iter().find(|i| i.item_id == item_id)
    }

    /// Mutable look-up by id.
    pub fn find_item_mut(&mut self, item_id: u32) -> Option<&mut ShopItem> {
        self.items.iter_mut().find(|i| i.item_id == item_id)
    }
}

// ---------------------------------------------------------------------------
// Vendor types
// ---------------------------------------------------------------------------

/// The various NPC vendor professions found across Britannia.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VendorType {
    Provisioner,
    Mage,
    Healer,
    Weaponsmith,
    Armorer,
    Blacksmith,
    Tailor,
    Tanner,
    Carpenter,
    Bowyer,
    Baker,
    Butcher,
    Alchemist,
    Scribe,
    Jeweler,
    Tinker,
    Innkeeper,
    Fisherman,
    Shipwright,
}

// ---------------------------------------------------------------------------
// Vendor definition (template)
// ---------------------------------------------------------------------------

/// A template that describes a particular vendor NPC, including its type,
/// buy/sell markup percentages, and its starting inventory.
#[derive(Debug, Clone)]
pub struct VendorDefinition {
    pub vendor_type: VendorType,
    pub title: String,
    /// Markup applied when a player *buys* from this vendor (1.0 = no markup).
    pub buy_markup: f64,
    /// Markdown applied when a player *sells* to this vendor (1.0 = full price).
    pub sell_markdown: f64,
    /// Starting shop inventory.
    pub inventory: ShopInventory,
}

// ---------------------------------------------------------------------------
// Trade result
// ---------------------------------------------------------------------------

/// Outcome of a buy or sell transaction.
#[derive(Debug, Clone, PartialEq)]
pub enum TradeResult {
    /// Transaction succeeded; contains the total gold exchanged.
    Success { total_gold: u32 },
    /// The vendor does not stock this item.
    ItemNotFound,
    /// The vendor does not have enough stock.
    InsufficientStock,
    /// The player cannot afford the purchase.
    InsufficientGold,
    /// The player tried to sell zero or negative quantity.
    InvalidQuantity,
    /// The vendor's inventory is full and cannot accept more of this item.
    VendorInventoryFull,
}

// ---------------------------------------------------------------------------
// Pricing helpers
// ---------------------------------------------------------------------------

/// Calculate the price a player must pay to **buy** `quantity` units of the
/// item with the given `base_price` from a vendor with `buy_markup`.
///
/// The result is always at least 1 gold per unit.
pub fn calculate_buy_price(base_price: u32, quantity: u32, buy_markup: f64) -> u32 {
    let unit = (base_price as f64 * buy_markup).ceil().max(1.0) as u32;
    unit.saturating_mul(quantity)
}

/// Calculate the gold a player receives when **selling** `quantity` units of
/// an item with the given `base_price` to a vendor with `sell_markdown`.
///
/// The result is always at least 1 gold per unit.
pub fn calculate_sell_price(base_price: u32, quantity: u32, sell_markdown: f64) -> u32 {
    let unit = (base_price as f64 * sell_markdown).ceil().max(1.0) as u32;
    unit.saturating_mul(quantity)
}

// ---------------------------------------------------------------------------
// Transaction functions
// ---------------------------------------------------------------------------

/// Attempt to buy `quantity` units of `item_id` from the vendor.
///
/// * `inventory` – the vendor's current shop inventory (mutated on success).
/// * `player_gold` – mutable reference to the player's gold pouch (mutated on
///   success).
/// * `buy_markup` – the vendor's buy markup multiplier.
///
/// Returns a `TradeResult` indicating the outcome.
pub fn attempt_buy(
    inventory: &mut ShopInventory,
    item_id: u32,
    quantity: u32,
    player_gold: &mut u32,
    buy_markup: f64,
) -> TradeResult {
    if quantity == 0 {
        return TradeResult::InvalidQuantity;
    }

    let item = match inventory.find_item(item_id) {
        Some(i) => i,
        None => return TradeResult::ItemNotFound,
    };

    if item.quantity < quantity {
        return TradeResult::InsufficientStock;
    }

    let total_cost = calculate_buy_price(item.base_price, quantity, buy_markup);

    if *player_gold < total_cost {
        return TradeResult::InsufficientGold;
    }

    // Commit the transaction.
    let item_mut = inventory.find_item_mut(item_id).unwrap();
    item_mut.quantity -= quantity;
    *player_gold -= total_cost;

    TradeResult::Success {
        total_gold: total_cost,
    }
}

/// Attempt to sell `quantity` units of `item_id` to the vendor.
///
/// * `inventory` – the vendor's current shop inventory (mutated on success).
/// * `player_gold` – mutable reference to the player's gold pouch (mutated on
///   success).
/// * `sell_markdown` – the vendor's sell markdown multiplier.
/// * `base_price` – the base price of the item being sold (looked up by the
///   caller from the game's item database).
///
/// If the vendor already stocks this item the quantity is increased; otherwise
/// the item is added to the vendor's inventory.
pub fn attempt_sell(
    inventory: &mut ShopInventory,
    item_id: u32,
    quantity: u32,
    player_gold: &mut u32,
    sell_markdown: f64,
    base_price: u32,
) -> TradeResult {
    if quantity == 0 {
        return TradeResult::InvalidQuantity;
    }

    // If the vendor already has this item, make sure there is room.
    if let Some(existing) = inventory.find_item(item_id) {
        if existing.quantity + quantity > existing.max_quantity {
            return TradeResult::VendorInventoryFull;
        }
    }

    let payout = calculate_sell_price(base_price, quantity, sell_markdown);

    // Commit the transaction.
    if let Some(existing) = inventory.find_item_mut(item_id) {
        existing.quantity += quantity;
    } else {
        // Vendor didn't stock it before – create an entry with a reasonable
        // max_quantity.
        inventory.add_item(ShopItem {
            item_id,
            name: String::from("Player-sold item"),
            base_price,
            quantity,
            max_quantity: 999,
            weight: 10, // default 1.0 stone
        });
    }

    *player_gold += payout;

    TradeResult::Success {
        total_gold: payout,
    }
}

// ---------------------------------------------------------------------------
// Pre-built vendor templates
// ---------------------------------------------------------------------------

/// Create a **Provisioner** vendor with typical general-goods inventory.
pub fn provisioner_template() -> VendorDefinition {
    let mut inv = ShopInventory::new();
    inv.add_item(ShopItem {
        item_id: 0x1F03,
        name: String::from("Backpack"),
        base_price: 15,
        quantity: 20,
        max_quantity: 50,
        weight: 30,
    });
    inv.add_item(ShopItem {
        item_id: 0x0A12,
        name: String::from("Torch"),
        base_price: 8,
        quantity: 30,
        max_quantity: 100,
        weight: 10,
    });
    inv.add_item(ShopItem {
        item_id: 0x0E76,
        name: String::from("Bedroll"),
        base_price: 5,
        quantity: 15,
        max_quantity: 40,
        weight: 50,
    });
    inv.add_item(ShopItem {
        item_id: 0x0F3F,
        name: String::from("Arrow"),
        base_price: 2,
        quantity: 100,
        max_quantity: 500,
        weight: 1,
    });
    inv.add_item(ShopItem {
        item_id: 0x1F4C,
        name: String::from("Bolt"),
        base_price: 3,
        quantity: 100,
        max_quantity: 500,
        weight: 1,
    });

    VendorDefinition {
        vendor_type: VendorType::Provisioner,
        title: String::from("the Provisioner"),
        buy_markup: 1.50,
        sell_markdown: 0.40,
        inventory: inv,
    }
}

/// Create a **Mage** vendor stocked with reagents and scrolls.
pub fn mage_template() -> VendorDefinition {
    let mut inv = ShopInventory::new();
    inv.add_item(ShopItem {
        item_id: 0x0F7A,
        name: String::from("Black Pearl"),
        base_price: 5,
        quantity: 50,
        max_quantity: 200,
        weight: 1,
    });
    inv.add_item(ShopItem {
        item_id: 0x0F7B,
        name: String::from("Blood Moss"),
        base_price: 5,
        quantity: 50,
        max_quantity: 200,
        weight: 1,
    });
    inv.add_item(ShopItem {
        item_id: 0x0F84,
        name: String::from("Garlic"),
        base_price: 3,
        quantity: 50,
        max_quantity: 200,
        weight: 1,
    });
    inv.add_item(ShopItem {
        item_id: 0x0F85,
        name: String::from("Ginseng"),
        base_price: 3,
        quantity: 50,
        max_quantity: 200,
        weight: 1,
    });
    inv.add_item(ShopItem {
        item_id: 0x0F86,
        name: String::from("Mandrake Root"),
        base_price: 3,
        quantity: 50,
        max_quantity: 200,
        weight: 1,
    });
    inv.add_item(ShopItem {
        item_id: 0x0F88,
        name: String::from("Nightshade"),
        base_price: 3,
        quantity: 50,
        max_quantity: 200,
        weight: 1,
    });
    inv.add_item(ShopItem {
        item_id: 0x0F8C,
        name: String::from("Sulfurous Ash"),
        base_price: 3,
        quantity: 50,
        max_quantity: 200,
        weight: 1,
    });
    inv.add_item(ShopItem {
        item_id: 0x0F8D,
        name: String::from("Spider's Silk"),
        base_price: 3,
        quantity: 50,
        max_quantity: 200,
        weight: 1,
    });
    inv.add_item(ShopItem {
        item_id: 0x1F4D,
        name: String::from("Recall Scroll"),
        base_price: 25,
        quantity: 10,
        max_quantity: 30,
        weight: 1,
    });

    VendorDefinition {
        vendor_type: VendorType::Mage,
        title: String::from("the Mage"),
        buy_markup: 1.60,
        sell_markdown: 0.35,
        inventory: inv,
    }
}

/// Create a **Healer** vendor with bandages and cure potions.
pub fn healer_template() -> VendorDefinition {
    let mut inv = ShopInventory::new();
    inv.add_item(ShopItem {
        item_id: 0x0E21,
        name: String::from("Bandage"),
        base_price: 5,
        quantity: 100,
        max_quantity: 500,
        weight: 1,
    });
    inv.add_item(ShopItem {
        item_id: 0x0F07,
        name: String::from("Greater Heal Potion"),
        base_price: 30,
        quantity: 15,
        max_quantity: 50,
        weight: 10,
    });
    inv.add_item(ShopItem {
        item_id: 0x0F08,
        name: String::from("Cure Potion"),
        base_price: 20,
        quantity: 15,
        max_quantity: 50,
        weight: 10,
    });
    inv.add_item(ShopItem {
        item_id: 0x0F09,
        name: String::from("Lesser Heal Potion"),
        base_price: 10,
        quantity: 20,
        max_quantity: 80,
        weight: 10,
    });
    inv.add_item(ShopItem {
        item_id: 0x0F0B,
        name: String::from("Resurrection Scroll"),
        base_price: 250,
        quantity: 3,
        max_quantity: 10,
        weight: 1,
    });

    VendorDefinition {
        vendor_type: VendorType::Healer,
        title: String::from("the Healer"),
        buy_markup: 1.40,
        sell_markdown: 0.45,
        inventory: inv,
    }
}

/// Create a **Weaponsmith** vendor with melee weapons.
pub fn weaponsmith_template() -> VendorDefinition {
    let mut inv = ShopInventory::new();
    inv.add_item(ShopItem {
        item_id: 0x0F5E,
        name: String::from("Broadsword"),
        base_price: 35,
        quantity: 5,
        max_quantity: 15,
        weight: 60,
    });
    inv.add_item(ShopItem {
        item_id: 0x13B9,
        name: String::from("Viking Sword"),
        base_price: 50,
        quantity: 3,
        max_quantity: 10,
        weight: 60,
    });
    inv.add_item(ShopItem {
        item_id: 0x0F43,
        name: String::from("Hatchet"),
        base_price: 20,
        quantity: 8,
        max_quantity: 20,
        weight: 40,
    });
    inv.add_item(ShopItem {
        item_id: 0x0F51,
        name: String::from("Dagger"),
        base_price: 15,
        quantity: 10,
        max_quantity: 25,
        weight: 10,
    });
    inv.add_item(ShopItem {
        item_id: 0x13FE,
        name: String::from("Katana"),
        base_price: 45,
        quantity: 4,
        max_quantity: 12,
        weight: 60,
    });
    inv.add_item(ShopItem {
        item_id: 0x0DF0,
        name: String::from("War Mace"),
        base_price: 40,
        quantity: 4,
        max_quantity: 12,
        weight: 80,
    });

    VendorDefinition {
        vendor_type: VendorType::Weaponsmith,
        title: String::from("the Weaponsmith"),
        buy_markup: 1.55,
        sell_markdown: 0.35,
        inventory: inv,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Pricing tests ------------------------------------------------------

    #[test]
    fn buy_price_with_no_markup() {
        assert_eq!(calculate_buy_price(10, 1, 1.0), 10);
        assert_eq!(calculate_buy_price(10, 5, 1.0), 50);
    }

    #[test]
    fn buy_price_with_markup() {
        // 10 gold * 1.5 markup = 15 per unit
        assert_eq!(calculate_buy_price(10, 1, 1.5), 15);
        assert_eq!(calculate_buy_price(10, 3, 1.5), 45);
    }

    #[test]
    fn buy_price_rounds_up_per_unit() {
        // 7 * 1.5 = 10.5 -> ceil -> 11 per unit
        assert_eq!(calculate_buy_price(7, 1, 1.5), 11);
        assert_eq!(calculate_buy_price(7, 2, 1.5), 22);
    }

    #[test]
    fn buy_price_minimum_one_gold() {
        assert_eq!(calculate_buy_price(0, 1, 1.5), 1);
    }

    #[test]
    fn sell_price_with_markdown() {
        // 10 * 0.4 = 4 per unit
        assert_eq!(calculate_sell_price(10, 1, 0.4), 4);
        assert_eq!(calculate_sell_price(10, 5, 0.4), 20);
    }

    #[test]
    fn sell_price_rounds_up() {
        // 7 * 0.4 = 2.8 -> ceil -> 3
        assert_eq!(calculate_sell_price(7, 1, 0.4), 3);
    }

    #[test]
    fn sell_price_minimum_one_gold() {
        assert_eq!(calculate_sell_price(1, 1, 0.01), 1);
    }

    // -- ShopInventory tests ------------------------------------------------

    #[test]
    fn add_item_new() {
        let mut inv = ShopInventory::new();
        inv.add_item(ShopItem {
            item_id: 1,
            name: String::from("Test"),
            base_price: 10,
            quantity: 5,
            max_quantity: 20,
            weight: 10,
        });
        assert_eq!(inv.items.len(), 1);
        assert_eq!(inv.items[0].quantity, 5);
    }

    #[test]
    fn add_item_merges_duplicates() {
        let mut inv = ShopInventory::new();
        inv.add_item(ShopItem {
            item_id: 1,
            name: String::from("Test"),
            base_price: 10,
            quantity: 5,
            max_quantity: 20,
            weight: 10,
        });
        inv.add_item(ShopItem {
            item_id: 1,
            name: String::from("Test"),
            base_price: 10,
            quantity: 10,
            max_quantity: 20,
            weight: 10,
        });
        assert_eq!(inv.items.len(), 1);
        assert_eq!(inv.items[0].quantity, 15);
    }

    #[test]
    fn add_item_clamps_to_max() {
        let mut inv = ShopInventory::new();
        inv.add_item(ShopItem {
            item_id: 1,
            name: String::from("Test"),
            base_price: 10,
            quantity: 15,
            max_quantity: 20,
            weight: 10,
        });
        inv.add_item(ShopItem {
            item_id: 1,
            name: String::from("Test"),
            base_price: 10,
            quantity: 100,
            max_quantity: 20,
            weight: 10,
        });
        assert_eq!(inv.items[0].quantity, 20);
    }

    #[test]
    fn find_item_returns_none_when_missing() {
        let inv = ShopInventory::new();
        assert!(inv.find_item(42).is_none());
    }

    // -- attempt_buy tests --------------------------------------------------

    #[test]
    fn buy_success() {
        let mut inv = ShopInventory::new();
        inv.add_item(ShopItem {
            item_id: 1,
            name: String::from("Sword"),
            base_price: 10,
            quantity: 5,
            max_quantity: 20,
            weight: 30,
        });
        let mut gold = 100;
        let result = attempt_buy(&mut inv, 1, 2, &mut gold, 1.5);
        // 10 * 1.5 = 15 per unit, 2 units = 30 gold
        assert_eq!(result, TradeResult::Success { total_gold: 30 });
        assert_eq!(gold, 70);
        assert_eq!(inv.find_item(1).unwrap().quantity, 3);
    }

    #[test]
    fn buy_item_not_found() {
        let mut inv = ShopInventory::new();
        let mut gold = 100;
        assert_eq!(
            attempt_buy(&mut inv, 999, 1, &mut gold, 1.0),
            TradeResult::ItemNotFound
        );
    }

    #[test]
    fn buy_insufficient_stock() {
        let mut inv = ShopInventory::new();
        inv.add_item(ShopItem {
            item_id: 1,
            name: String::from("Sword"),
            base_price: 10,
            quantity: 1,
            max_quantity: 20,
            weight: 30,
        });
        let mut gold = 1000;
        assert_eq!(
            attempt_buy(&mut inv, 1, 5, &mut gold, 1.0),
            TradeResult::InsufficientStock
        );
        // gold and quantity unchanged
        assert_eq!(gold, 1000);
        assert_eq!(inv.find_item(1).unwrap().quantity, 1);
    }

    #[test]
    fn buy_insufficient_gold() {
        let mut inv = ShopInventory::new();
        inv.add_item(ShopItem {
            item_id: 1,
            name: String::from("Sword"),
            base_price: 100,
            quantity: 5,
            max_quantity: 20,
            weight: 30,
        });
        let mut gold = 10;
        assert_eq!(
            attempt_buy(&mut inv, 1, 1, &mut gold, 1.0),
            TradeResult::InsufficientGold
        );
        assert_eq!(gold, 10);
    }

    #[test]
    fn buy_zero_quantity() {
        let mut inv = ShopInventory::new();
        inv.add_item(ShopItem {
            item_id: 1,
            name: String::from("Sword"),
            base_price: 10,
            quantity: 5,
            max_quantity: 20,
            weight: 30,
        });
        let mut gold = 100;
        assert_eq!(
            attempt_buy(&mut inv, 1, 0, &mut gold, 1.0),
            TradeResult::InvalidQuantity
        );
    }

    // -- attempt_sell tests -------------------------------------------------

    #[test]
    fn sell_success_existing_item() {
        let mut inv = ShopInventory::new();
        inv.add_item(ShopItem {
            item_id: 1,
            name: String::from("Sword"),
            base_price: 10,
            quantity: 5,
            max_quantity: 20,
            weight: 30,
        });
        let mut gold = 0;
        let result = attempt_sell(&mut inv, 1, 3, &mut gold, 0.5, 10);
        // 10 * 0.5 = 5 per unit, 3 units = 15 gold
        assert_eq!(result, TradeResult::Success { total_gold: 15 });
        assert_eq!(gold, 15);
        assert_eq!(inv.find_item(1).unwrap().quantity, 8);
    }

    #[test]
    fn sell_success_new_item() {
        let mut inv = ShopInventory::new();
        let mut gold = 0;
        let result = attempt_sell(&mut inv, 42, 2, &mut gold, 0.5, 20);
        // 20 * 0.5 = 10 per unit, 2 units = 20 gold
        assert_eq!(result, TradeResult::Success { total_gold: 20 });
        assert_eq!(gold, 20);
        assert_eq!(inv.find_item(42).unwrap().quantity, 2);
    }

    #[test]
    fn sell_vendor_full() {
        let mut inv = ShopInventory::new();
        inv.add_item(ShopItem {
            item_id: 1,
            name: String::from("Sword"),
            base_price: 10,
            quantity: 19,
            max_quantity: 20,
            weight: 30,
        });
        let mut gold = 0;
        assert_eq!(
            attempt_sell(&mut inv, 1, 5, &mut gold, 0.5, 10),
            TradeResult::VendorInventoryFull
        );
        assert_eq!(gold, 0);
        assert_eq!(inv.find_item(1).unwrap().quantity, 19);
    }

    #[test]
    fn sell_zero_quantity() {
        let mut inv = ShopInventory::new();
        let mut gold = 0;
        assert_eq!(
            attempt_sell(&mut inv, 1, 0, &mut gold, 0.5, 10),
            TradeResult::InvalidQuantity
        );
    }

    // -- Vendor template tests ----------------------------------------------

    #[test]
    fn provisioner_template_is_valid() {
        let v = provisioner_template();
        assert_eq!(v.vendor_type, VendorType::Provisioner);
        assert!(v.buy_markup > 1.0);
        assert!(v.sell_markdown < 1.0);
        assert!(!v.inventory.items.is_empty());
        // Backpack should be present
        assert!(v.inventory.find_item(0x1F03).is_some());
    }

    #[test]
    fn mage_template_is_valid() {
        let v = mage_template();
        assert_eq!(v.vendor_type, VendorType::Mage);
        assert!(!v.inventory.items.is_empty());
        // Black Pearl should be present
        assert!(v.inventory.find_item(0x0F7A).is_some());
    }

    #[test]
    fn healer_template_is_valid() {
        let v = healer_template();
        assert_eq!(v.vendor_type, VendorType::Healer);
        assert!(!v.inventory.items.is_empty());
        // Bandage should be present
        assert!(v.inventory.find_item(0x0E21).is_some());
    }

    #[test]
    fn weaponsmith_template_is_valid() {
        let v = weaponsmith_template();
        assert_eq!(v.vendor_type, VendorType::Weaponsmith);
        assert!(!v.inventory.items.is_empty());
        // Broadsword should be present
        assert!(v.inventory.find_item(0x0F5E).is_some());
    }

    #[test]
    fn all_vendor_types_exist() {
        // Ensure all 19 enum variants can be instantiated without panic.
        let types = vec![
            VendorType::Provisioner,
            VendorType::Mage,
            VendorType::Healer,
            VendorType::Weaponsmith,
            VendorType::Armorer,
            VendorType::Blacksmith,
            VendorType::Tailor,
            VendorType::Tanner,
            VendorType::Carpenter,
            VendorType::Bowyer,
            VendorType::Baker,
            VendorType::Butcher,
            VendorType::Alchemist,
            VendorType::Scribe,
            VendorType::Jeweler,
            VendorType::Tinker,
            VendorType::Innkeeper,
            VendorType::Fisherman,
            VendorType::Shipwright,
        ];
        assert_eq!(types.len(), 19);
    }

    // -- Integration: buy from a template vendor ----------------------------

    #[test]
    fn buy_from_provisioner() {
        let mut v = provisioner_template();
        let mut gold = 500;
        let backpack_id = 0x1F03;
        let result = attempt_buy(
            &mut v.inventory,
            backpack_id,
            2,
            &mut gold,
            v.buy_markup,
        );
        // base 15 * 1.50 = 22.5 -> ceil -> 23 per unit, 2 units = 46
        assert_eq!(result, TradeResult::Success { total_gold: 46 });
        assert_eq!(gold, 454);
    }

    #[test]
    fn sell_to_weaponsmith() {
        let mut v = weaponsmith_template();
        let mut gold = 0;
        let broadsword_id = 0x0F5E;
        let base_price = 35;
        let result = attempt_sell(
            &mut v.inventory,
            broadsword_id,
            1,
            &mut gold,
            v.sell_markdown,
            base_price,
        );
        // 35 * 0.35 = 12.25 -> ceil -> 13
        assert_eq!(result, TradeResult::Success { total_gold: 13 });
        assert_eq!(gold, 13);
    }
}
