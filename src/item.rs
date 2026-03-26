use std::fmt;

/// Unique identifier for all game objects (items, mobiles, etc.).
pub type Serial = u32;

/// Equipment layer on a mobile (character or NPC).
/// Values mirror the classic UO protocol layer byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ItemLayer {
    RightHand = 1,
    LeftHand = 2,
    Shoes = 3,
    Pants = 4,
    Shirt = 5,
    Helm = 6,
    Gloves = 7,
    Ring = 8,
    Neck = 10,
    Hair = 11,
    Waist = 12,
    Chest = 13,
    Bracelet = 14,
    FacialHair = 16,
    Cloak = 20,
    Backpack = 21,
    Robe = 22,
    Earrings = 24,
    Arms = 25,
    Mount = 26,
}

impl fmt::Display for ItemLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

bitflags::bitflags! {
    /// Flags that describe item behaviour and state.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ItemFlags: u16 {
        const MOVABLE    = 0b0000_0001;
        const VISIBLE    = 0b0000_0010;
        const STACKABLE  = 0b0000_0100;
        const BLESSED    = 0b0000_1000;
        const INSURED    = 0b0001_0000;
        const NEWBIED    = 0b0010_0000;
        const LOCKED_DOWN = 0b0100_0000;
        const SECURED    = 0b1000_0000;
    }
}

impl Default for ItemFlags {
    fn default() -> Self {
        ItemFlags::MOVABLE | ItemFlags::VISIBLE
    }
}

/// A single in-game item.
#[derive(Debug, Clone)]
pub struct Item {
    /// Globally unique serial number.
    pub serial: Serial,
    /// Graphic / art tile id.
    pub item_id: u16,
    /// Display name.
    pub name: String,
    /// Colour hue (0 = default).
    pub hue: u16,
    /// Stack count (1 for non-stackables).
    pub amount: u16,
    /// Weight per single unit in stones.
    pub weight: f32,
    /// World position (x, y, z). `None` when the item lives inside a container.
    pub position: Option<(u16, u16, i8)>,
    /// Serial of the parent container or mobile, `None` when on the ground.
    pub parent_serial: Option<Serial>,
    /// Equipment layer when worn/equipped on a mobile.
    pub layer: Option<ItemLayer>,
    /// Behavioural flags.
    pub flags: ItemFlags,
}

impl Item {
    /// Create a new item with sensible defaults.
    pub fn new(serial: Serial, item_id: u16, name: impl Into<String>) -> Self {
        Self {
            serial,
            item_id,
            name: name.into(),
            hue: 0,
            amount: 1,
            weight: 1.0,
            position: None,
            parent_serial: None,
            layer: None,
            flags: ItemFlags::default(),
        }
    }

    /// Whether the item is currently equipped on a mobile.
    pub fn is_equipped(&self) -> bool {
        self.layer.is_some() && self.parent_serial.is_some()
    }

    /// Whether the item is inside a container (not equipped).
    pub fn is_in_container(&self) -> bool {
        self.parent_serial.is_some() && self.layer.is_none()
    }

    /// Whether the item is lying on the ground.
    pub fn is_on_ground(&self) -> bool {
        self.position.is_some() && self.parent_serial.is_none()
    }

    /// Total weight taking stack amount into account.
    pub fn total_weight(&self) -> f32 {
        self.weight * self.amount as f32
    }
}

/// A container adds capacity semantics on top of the base `Item`.
#[derive(Debug, Clone)]
pub struct Container {
    /// The underlying item data.
    pub item: Item,
    /// Maximum number of child items this container can hold.
    pub max_items: u16,
    /// Maximum total weight (in stones) the container can hold.
    pub max_weight: f32,
}

impl Container {
    pub fn new(serial: Serial, item_id: u16, name: impl Into<String>) -> Self {
        Self {
            item: Item::new(serial, item_id, name),
            max_items: 125,
            max_weight: 400.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_item_has_default_flags() {
        let item = Item::new(1, 0x0F51, "Longsword");
        assert!(item.flags.contains(ItemFlags::MOVABLE));
        assert!(item.flags.contains(ItemFlags::VISIBLE));
        assert!(!item.flags.contains(ItemFlags::STACKABLE));
    }

    #[test]
    fn item_is_on_ground_when_placed_in_world() {
        let mut item = Item::new(1, 0x0F51, "Longsword");
        item.position = Some((1000, 2000, 0));
        assert!(item.is_on_ground());
        assert!(!item.is_equipped());
        assert!(!item.is_in_container());
    }

    #[test]
    fn item_is_equipped_when_has_layer_and_parent() {
        let mut item = Item::new(1, 0x0F51, "Longsword");
        item.parent_serial = Some(0x0000_0001);
        item.layer = Some(ItemLayer::RightHand);
        assert!(item.is_equipped());
        assert!(!item.is_on_ground());
        assert!(!item.is_in_container());
    }

    #[test]
    fn item_is_in_container_when_has_parent_but_no_layer() {
        let mut item = Item::new(2, 0x0EED, "Gold Coin");
        item.parent_serial = Some(100);
        assert!(item.is_in_container());
        assert!(!item.is_equipped());
    }

    #[test]
    fn total_weight_accounts_for_stack_amount() {
        let mut item = Item::new(3, 0x0EED, "Gold Coin");
        item.weight = 0.1;
        item.amount = 50;
        item.flags |= ItemFlags::STACKABLE;
        let expected = 0.1_f32 * 50.0;
        assert!((item.total_weight() - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn container_has_sane_defaults() {
        let c = Container::new(10, 0x0E75, "Backpack");
        assert_eq!(c.max_items, 125);
        assert!((c.max_weight - 400.0).abs() < f32::EPSILON);
        assert_eq!(c.item.serial, 10);
    }

    #[test]
    fn item_layer_display() {
        assert_eq!(format!("{}", ItemLayer::RightHand), "RightHand");
        assert_eq!(format!("{}", ItemLayer::Backpack), "Backpack");
    }
}
