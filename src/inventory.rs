use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::item::{Item, ItemLayer, Serial};

/// Starting serial for items. In classic UO the item serial range begins at
/// 0x4000_0000 (mobiles occupy the lower range).
const ITEM_SERIAL_START: u32 = 0x4000_0000;

/// Global atomic counter used to generate unique item serials.
static NEXT_SERIAL: AtomicU32 = AtomicU32::new(ITEM_SERIAL_START);

/// Generate the next unique item serial.
pub fn next_serial() -> Serial {
    NEXT_SERIAL.fetch_add(1, Ordering::Relaxed)
}

/// Reset the serial counter (useful for tests).
#[cfg(test)]
fn reset_serial_counter() {
    NEXT_SERIAL.store(ITEM_SERIAL_START, Ordering::Relaxed);
}

/// Central storage for all items in the game world.
#[derive(Debug, Default)]
pub struct Inventory {
    items: HashMap<Serial, Item>,
}

/// Errors that can occur during inventory operations.
#[derive(Debug, PartialEq, Eq)]
pub enum InventoryError {
    /// An item with the given serial already exists.
    DuplicateSerial(Serial),
    /// No item with the given serial was found.
    NotFound(Serial),
}

impl std::fmt::Display for InventoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InventoryError::DuplicateSerial(s) => {
                write!(f, "item with serial 0x{:08X} already exists", s)
            }
            InventoryError::NotFound(s) => {
                write!(f, "item with serial 0x{:08X} not found", s)
            }
        }
    }
}

impl Inventory {
    /// Create an empty inventory.
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }

    /// Total number of tracked items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the inventory is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Insert an item. Returns an error if the serial is already taken.
    pub fn add_item(&mut self, item: Item) -> Result<Serial, InventoryError> {
        let serial = item.serial;
        if self.items.contains_key(&serial) {
            return Err(InventoryError::DuplicateSerial(serial));
        }
        self.items.insert(serial, item);
        Ok(serial)
    }

    /// Remove and return an item by serial.
    pub fn remove_item(&mut self, serial: Serial) -> Result<Item, InventoryError> {
        self.items
            .remove(&serial)
            .ok_or(InventoryError::NotFound(serial))
    }

    /// Get an immutable reference to an item.
    pub fn get_item(&self, serial: Serial) -> Option<&Item> {
        self.items.get(&serial)
    }

    /// Get a mutable reference to an item.
    pub fn get_item_mut(&mut self, serial: Serial) -> Option<&mut Item> {
        self.items.get_mut(&serial)
    }

    /// Return all items whose `parent_serial` equals the given container serial
    /// and that are **not** equipped (i.e. `layer` is `None`).
    pub fn get_items_in_container(&self, container_serial: Serial) -> Vec<&Item> {
        self.items
            .values()
            .filter(|item| {
                item.parent_serial == Some(container_serial) && item.layer.is_none()
            })
            .collect()
    }

    /// Return all items equipped on a mobile (have both `parent_serial` and
    /// `layer` set).
    pub fn get_equipped_items(&self, mobile_serial: Serial) -> Vec<&Item> {
        self.items
            .values()
            .filter(|item| {
                item.parent_serial == Some(mobile_serial) && item.layer.is_some()
            })
            .collect()
    }

    /// Get the equipped item in a specific layer on a mobile.
    pub fn get_equipped_in_layer(
        &self,
        mobile_serial: Serial,
        layer: ItemLayer,
    ) -> Option<&Item> {
        self.items.values().find(|item| {
            item.parent_serial == Some(mobile_serial) && item.layer == Some(layer)
        })
    }

    /// Compute the total weight of all items inside a container (non-recursive).
    pub fn container_weight(&self, container_serial: Serial) -> f32 {
        self.get_items_in_container(container_serial)
            .iter()
            .map(|item| item.total_weight())
            .sum()
    }

    /// Compute the total weight of every item in the inventory.
    pub fn total_weight(&self) -> f32 {
        self.items.values().map(|item| item.total_weight()).sum()
    }

    /// Recursively compute the weight of a container and everything inside it.
    pub fn recursive_weight(&self, container_serial: Serial) -> f32 {
        let own_weight = self
            .get_item(container_serial)
            .map_or(0.0, |i| i.total_weight());

        let children_weight: f32 = self
            .get_items_in_container(container_serial)
            .iter()
            .map(|child| {
                // If the child itself is a container, recurse.
                let has_children = self
                    .items
                    .values()
                    .any(|i| i.parent_serial == Some(child.serial));
                if has_children {
                    self.recursive_weight(child.serial)
                } else {
                    child.total_weight()
                }
            })
            .sum();

        own_weight + children_weight
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::{ItemFlags, ItemLayer};

    /// Helper: create a simple item with a given serial and name.
    fn make_item(serial: Serial, name: &str) -> Item {
        Item::new(serial, 0x0F51, name)
    }

    // -----------------------------------------------------------------------
    // Serial generation
    // -----------------------------------------------------------------------

    #[test]
    fn serial_generation_is_monotonic() {
        let a = next_serial();
        let b = next_serial();
        assert!(b > a, "serials must be strictly increasing");
    }

    // -----------------------------------------------------------------------
    // Basic CRUD
    // -----------------------------------------------------------------------

    #[test]
    fn add_and_get_item() {
        let mut inv = Inventory::new();
        let item = make_item(1, "Sword");
        inv.add_item(item).unwrap();
        assert_eq!(inv.len(), 1);

        let fetched = inv.get_item(1).unwrap();
        assert_eq!(fetched.name, "Sword");
    }

    #[test]
    fn add_duplicate_serial_errors() {
        let mut inv = Inventory::new();
        inv.add_item(make_item(1, "Sword")).unwrap();
        let result = inv.add_item(make_item(1, "Shield"));
        assert_eq!(result, Err(InventoryError::DuplicateSerial(1)));
    }

    #[test]
    fn remove_item_returns_it() {
        let mut inv = Inventory::new();
        inv.add_item(make_item(1, "Sword")).unwrap();
        let removed = inv.remove_item(1).unwrap();
        assert_eq!(removed.name, "Sword");
        assert!(inv.is_empty());
    }

    #[test]
    fn remove_missing_item_errors() {
        let mut inv = Inventory::new();
        assert_eq!(inv.remove_item(999), Err(InventoryError::NotFound(999)));
    }

    #[test]
    fn get_item_mut_allows_modification() {
        let mut inv = Inventory::new();
        inv.add_item(make_item(1, "Sword")).unwrap();
        inv.get_item_mut(1).unwrap().hue = 0x0044;
        assert_eq!(inv.get_item(1).unwrap().hue, 0x0044);
    }

    // -----------------------------------------------------------------------
    // Container queries
    // -----------------------------------------------------------------------

    #[test]
    fn get_items_in_container() {
        let mut inv = Inventory::new();

        // A backpack with serial 100
        let mut backpack = make_item(100, "Backpack");
        backpack.item_id = 0x0E75;
        inv.add_item(backpack).unwrap();

        // Two items inside the backpack
        let mut gold = Item::new(101, 0x0EED, "Gold");
        gold.parent_serial = Some(100);
        gold.amount = 500;
        gold.weight = 0.02;
        gold.flags |= ItemFlags::STACKABLE;
        inv.add_item(gold).unwrap();

        let mut gem = Item::new(102, 0x0F26, "Diamond");
        gem.parent_serial = Some(100);
        inv.add_item(gem).unwrap();

        // An equipped item on mobile 100 should NOT show up
        let mut equipped = Item::new(103, 0x0F51, "Equipped Sword");
        equipped.parent_serial = Some(100);
        equipped.layer = Some(ItemLayer::RightHand);
        inv.add_item(equipped).unwrap();

        let contents = inv.get_items_in_container(100);
        assert_eq!(contents.len(), 2);
    }

    // -----------------------------------------------------------------------
    // Equipment queries
    // -----------------------------------------------------------------------

    #[test]
    fn get_equipped_items_on_mobile() {
        let mut inv = Inventory::new();
        let mobile_serial: Serial = 1;

        let mut sword = Item::new(200, 0x0F51, "Longsword");
        sword.parent_serial = Some(mobile_serial);
        sword.layer = Some(ItemLayer::RightHand);
        inv.add_item(sword).unwrap();

        let mut shield = Item::new(201, 0x1B76, "Wooden Shield");
        shield.parent_serial = Some(mobile_serial);
        shield.layer = Some(ItemLayer::LeftHand);
        inv.add_item(shield).unwrap();

        // Item in backpack (not equipped)
        let mut potion = Item::new(202, 0x0F06, "Heal Potion");
        potion.parent_serial = Some(mobile_serial);
        inv.add_item(potion).unwrap();

        let equipped = inv.get_equipped_items(mobile_serial);
        assert_eq!(equipped.len(), 2);
    }

    #[test]
    fn get_equipped_in_specific_layer() {
        let mut inv = Inventory::new();
        let mobile: Serial = 1;

        let mut helm = Item::new(300, 0x1408, "Close Helm");
        helm.parent_serial = Some(mobile);
        helm.layer = Some(ItemLayer::Helm);
        inv.add_item(helm).unwrap();

        assert!(inv.get_equipped_in_layer(mobile, ItemLayer::Helm).is_some());
        assert!(inv
            .get_equipped_in_layer(mobile, ItemLayer::RightHand)
            .is_none());
    }

    // -----------------------------------------------------------------------
    // Weight calculations
    // -----------------------------------------------------------------------

    #[test]
    fn container_weight_sums_children() {
        let mut inv = Inventory::new();

        let backpack = make_item(100, "Backpack");
        inv.add_item(backpack).unwrap();

        let mut item_a = Item::new(101, 0x0F51, "Sword");
        item_a.parent_serial = Some(100);
        item_a.weight = 6.0;
        inv.add_item(item_a).unwrap();

        let mut item_b = Item::new(102, 0x0EED, "Gold");
        item_b.parent_serial = Some(100);
        item_b.weight = 0.02;
        item_b.amount = 100;
        inv.add_item(item_b).unwrap();

        let w = inv.container_weight(100);
        let expected = 6.0 + 0.02 * 100.0;
        assert!((w - expected).abs() < 0.001);
    }

    #[test]
    fn total_weight_sums_everything() {
        let mut inv = Inventory::new();

        let mut a = make_item(1, "A");
        a.weight = 3.0;
        inv.add_item(a).unwrap();

        let mut b = make_item(2, "B");
        b.weight = 7.0;
        inv.add_item(b).unwrap();

        assert!((inv.total_weight() - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn recursive_weight_includes_nested_containers() {
        let mut inv = Inventory::new();

        // Outer backpack
        let mut outer = Item::new(100, 0x0E75, "Outer Backpack");
        outer.weight = 3.0;
        inv.add_item(outer).unwrap();

        // Inner pouch inside outer
        let mut inner = Item::new(101, 0x0E79, "Pouch");
        inner.weight = 1.0;
        inner.parent_serial = Some(100);
        inv.add_item(inner).unwrap();

        // Gem inside inner pouch
        let mut gem = Item::new(102, 0x0F26, "Diamond");
        gem.weight = 0.5;
        gem.parent_serial = Some(101);
        inv.add_item(gem).unwrap();

        // Sword directly in outer backpack
        let mut sword = Item::new(103, 0x0F51, "Sword");
        sword.weight = 6.0;
        sword.parent_serial = Some(100);
        inv.add_item(sword).unwrap();

        // recursive_weight(outer) = outer(3) + sword(6) + inner_recursive(1 + 0.5)
        let w = inv.recursive_weight(100);
        let expected = 3.0 + 6.0 + 1.0 + 0.5;
        assert!(
            (w - expected).abs() < 0.001,
            "expected {}, got {}",
            expected,
            w
        );
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn empty_inventory() {
        let inv = Inventory::new();
        assert!(inv.is_empty());
        assert_eq!(inv.len(), 0);
        assert!(inv.total_weight().abs() < f32::EPSILON);
        assert!(inv.get_items_in_container(1).is_empty());
        assert!(inv.get_equipped_items(1).is_empty());
    }

    #[test]
    fn error_display_formats_nicely() {
        let e = InventoryError::DuplicateSerial(0x4000_0001);
        assert!(format!("{}", e).contains("0x40000001"));

        let e = InventoryError::NotFound(42);
        assert!(format!("{}", e).contains("0x0000002A"));
    }
}
