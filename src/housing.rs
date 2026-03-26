use std::collections::HashSet;
use std::time::{Duration, SystemTime};

/// All placeable house types in Ultima Online.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HouseType {
    SmallBrickHouse,
    SmallPlasterHouse,
    SmallFieldStoneHouse,
    SmallWoodHouse,
    SmallWoodAndPlasterHouse,
    SmallThatchedRoofCottage,
    TwoStoryWoodAndPlasterHouse,
    TwoStoryStoneAndPlasterHouse,
    LargeBrickHouse,
    LargePatioHouse,
    LargeMarbleHouse,
    Tower,
    SmallStoneTower,
    Castle,
    Keep,
}

/// Controls who may enter and interact with the house.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseSecurity {
    Public,
    Private,
    GuildOnly,
    OwnerOnly,
}

/// Decay lifecycle of a house.  Stages advance when the house is not
/// refreshed within the expected interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HouseDecayState {
    LikeNew,
    SlightlyWorn,
    SomewhatWorn,
    FairlyWorn,
    Greatly,
    InDanger,
    Condemned,
}

/// Static definition data shared by every house of the same type.
#[derive(Debug, Clone)]
pub struct HouseDefinition {
    pub house_type: HouseType,
    pub name: &'static str,
    pub max_lockdowns: u32,
    pub max_secures: u32,
    pub cost: u32,
    pub width: u32,
    pub height: u32,
    pub stories: u8,
}

impl HouseDefinition {
    /// Return the definition for a given house type.
    pub fn for_type(house_type: HouseType) -> &'static HouseDefinition {
        HOUSE_DEFINITIONS
            .iter()
            .find(|d| d.house_type == house_type)
            .expect("every HouseType must have a definition")
    }
}

static HOUSE_DEFINITIONS: &[HouseDefinition] = &[
    HouseDefinition {
        house_type: HouseType::SmallBrickHouse,
        name: "Small Brick House",
        max_lockdowns: 3,
        max_secures: 1,
        cost: 22_500,
        width: 7,
        height: 7,
        stories: 1,
    },
    HouseDefinition {
        house_type: HouseType::SmallPlasterHouse,
        name: "Small Plaster House",
        max_lockdowns: 3,
        max_secures: 1,
        cost: 22_500,
        width: 7,
        height: 7,
        stories: 1,
    },
    HouseDefinition {
        house_type: HouseType::SmallFieldStoneHouse,
        name: "Small Field Stone House",
        max_lockdowns: 3,
        max_secures: 1,
        cost: 22_500,
        width: 7,
        height: 7,
        stories: 1,
    },
    HouseDefinition {
        house_type: HouseType::SmallWoodHouse,
        name: "Small Wood House",
        max_lockdowns: 3,
        max_secures: 1,
        cost: 22_500,
        width: 7,
        height: 7,
        stories: 1,
    },
    HouseDefinition {
        house_type: HouseType::SmallWoodAndPlasterHouse,
        name: "Small Wood and Plaster House",
        max_lockdowns: 3,
        max_secures: 1,
        cost: 22_500,
        width: 7,
        height: 7,
        stories: 1,
    },
    HouseDefinition {
        house_type: HouseType::SmallThatchedRoofCottage,
        name: "Small Thatched Roof Cottage",
        max_lockdowns: 3,
        max_secures: 1,
        cost: 22_500,
        width: 7,
        height: 7,
        stories: 1,
    },
    HouseDefinition {
        house_type: HouseType::TwoStoryWoodAndPlasterHouse,
        name: "Two-Story Wood and Plaster House",
        max_lockdowns: 8,
        max_secures: 3,
        cost: 73_500,
        width: 9,
        height: 13,
        stories: 2,
    },
    HouseDefinition {
        house_type: HouseType::TwoStoryStoneAndPlasterHouse,
        name: "Two-Story Stone and Plaster House",
        max_lockdowns: 8,
        max_secures: 3,
        cost: 73_500,
        width: 9,
        height: 13,
        stories: 2,
    },
    HouseDefinition {
        house_type: HouseType::LargeBrickHouse,
        name: "Large Brick House",
        max_lockdowns: 14,
        max_secures: 5,
        cost: 131_500,
        width: 14,
        height: 14,
        stories: 2,
    },
    HouseDefinition {
        house_type: HouseType::LargePatioHouse,
        name: "Large Patio House",
        max_lockdowns: 14,
        max_secures: 5,
        cost: 131_500,
        width: 14,
        height: 14,
        stories: 2,
    },
    HouseDefinition {
        house_type: HouseType::LargeMarbleHouse,
        name: "Large Marble House",
        max_lockdowns: 14,
        max_secures: 5,
        cost: 152_000,
        width: 14,
        height: 14,
        stories: 2,
    },
    HouseDefinition {
        house_type: HouseType::Tower,
        name: "Tower",
        max_lockdowns: 28,
        max_secures: 9,
        cost: 433_200,
        width: 16,
        height: 14,
        stories: 4,
    },
    HouseDefinition {
        house_type: HouseType::SmallStoneTower,
        name: "Small Stone Tower",
        max_lockdowns: 8,
        max_secures: 3,
        cost: 73_500,
        width: 9,
        height: 9,
        stories: 3,
    },
    HouseDefinition {
        house_type: HouseType::Castle,
        name: "Castle",
        max_lockdowns: 56,
        max_secures: 16,
        cost: 865_250,
        width: 31,
        height: 31,
        stories: 3,
    },
    HouseDefinition {
        house_type: HouseType::Keep,
        name: "Keep",
        max_lockdowns: 28,
        max_secures: 9,
        cost: 665_200,
        width: 24,
        height: 24,
        stories: 3,
    },
];

/// World position for a placed house.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: u16,
    pub y: u16,
    pub z: i8,
    pub map: u8,
}

/// A placed house in the world.
#[derive(Debug)]
pub struct House {
    pub serial: u32,
    pub house_type: HouseType,
    pub owner: u32,
    pub co_owners: HashSet<u32>,
    pub friends: HashSet<u32>,
    pub bans: HashSet<u32>,
    pub position: Position,
    pub security: HouseSecurity,
    pub decay_state: HouseDecayState,
    pub locked_downs: HashSet<u32>,
    pub secures: HashSet<u32>,
    pub last_refresh: SystemTime,
}

impl House {
    /// Create a new house with default settings.
    pub fn new(serial: u32, house_type: HouseType, owner: u32, position: Position) -> Self {
        House {
            serial,
            house_type,
            owner,
            co_owners: HashSet::new(),
            friends: HashSet::new(),
            bans: HashSet::new(),
            position,
            security: HouseSecurity::Private,
            decay_state: HouseDecayState::LikeNew,
            locked_downs: HashSet::new(),
            secures: HashSet::new(),
            last_refresh: SystemTime::now(),
        }
    }

    // ---- co-owner management ----

    pub fn add_co_owner(&mut self, character_serial: u32) -> bool {
        if character_serial == self.owner {
            return false;
        }
        self.bans.remove(&character_serial);
        self.friends.remove(&character_serial);
        self.co_owners.insert(character_serial)
    }

    pub fn remove_co_owner(&mut self, character_serial: u32) -> bool {
        self.co_owners.remove(&character_serial)
    }

    // ---- friend management ----

    pub fn add_friend(&mut self, character_serial: u32) -> bool {
        if character_serial == self.owner || self.co_owners.contains(&character_serial) {
            return false;
        }
        self.bans.remove(&character_serial);
        self.friends.insert(character_serial)
    }

    pub fn remove_friend(&mut self, character_serial: u32) -> bool {
        self.friends.remove(&character_serial)
    }

    // ---- ban management ----

    pub fn ban(&mut self, character_serial: u32) -> bool {
        if character_serial == self.owner || self.co_owners.contains(&character_serial) {
            return false;
        }
        self.friends.remove(&character_serial);
        self.bans.insert(character_serial)
    }

    pub fn unban(&mut self, character_serial: u32) -> bool {
        self.bans.remove(&character_serial)
    }

    // ---- access check ----

    /// Determine whether `character_serial` (with optional `guild_id`)
    /// is allowed to access this house under its current security level.
    pub fn has_access(&self, character_serial: u32, guild_id: Option<u32>, owner_guild: Option<u32>) -> bool {
        if self.bans.contains(&character_serial) {
            return false;
        }
        if character_serial == self.owner
            || self.co_owners.contains(&character_serial)
            || self.friends.contains(&character_serial)
        {
            return true;
        }
        match self.security {
            HouseSecurity::Public => true,
            HouseSecurity::GuildOnly => {
                match (guild_id, owner_guild) {
                    (Some(g), Some(og)) => g == og,
                    _ => false,
                }
            }
            HouseSecurity::Private | HouseSecurity::OwnerOnly => false,
        }
    }

    // ---- lock-down / secure ----

    /// Lock down an item inside the house.
    /// Returns `false` if the lock-down limit has been reached or the item
    /// is already locked down.
    pub fn lock_down_item(&mut self, item_serial: u32) -> bool {
        let def = HouseDefinition::for_type(self.house_type);
        if self.locked_downs.len() as u32 >= def.max_lockdowns {
            return false;
        }
        self.locked_downs.insert(item_serial)
    }

    /// Release a locked-down item.
    pub fn release_item(&mut self, item_serial: u32) -> bool {
        self.locked_downs.remove(&item_serial)
    }

    /// Secure a container inside the house.
    /// Returns `false` if the secure limit has been reached or the container
    /// is already secured.
    pub fn secure_container(&mut self, container_serial: u32) -> bool {
        let def = HouseDefinition::for_type(self.house_type);
        if self.secures.len() as u32 >= def.max_secures {
            return false;
        }
        self.secures.insert(container_serial)
    }

    /// Release a secured container.
    pub fn release_container(&mut self, container_serial: u32) -> bool {
        self.secures.remove(&container_serial)
    }

    // ---- decay / refresh ----

    /// Refresh the house, resetting the decay timer back to `LikeNew`.
    pub fn refresh(&mut self) {
        self.decay_state = HouseDecayState::LikeNew;
        self.last_refresh = SystemTime::now();
    }

    /// Return the current decay level based on elapsed time since last
    /// refresh.  Each stage spans roughly one day; after seven days the
    /// house is condemned.
    pub fn decay_level(&self) -> HouseDecayState {
        let elapsed = self
            .last_refresh
            .elapsed()
            .unwrap_or(Duration::ZERO);

        let days = elapsed.as_secs() / 86_400;
        match days {
            0 => HouseDecayState::LikeNew,
            1 => HouseDecayState::SlightlyWorn,
            2 => HouseDecayState::SomewhatWorn,
            3 => HouseDecayState::FairlyWorn,
            4 => HouseDecayState::Greatly,
            5 => HouseDecayState::InDanger,
            _ => HouseDecayState::Condemned,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_position() -> Position {
        Position {
            x: 1000,
            y: 2000,
            z: 0,
            map: 0,
        }
    }

    fn sample_house() -> House {
        House::new(1, HouseType::SmallBrickHouse, 100, sample_position())
    }

    // ---- definition tests ----

    #[test]
    fn all_house_types_have_definitions() {
        let types = [
            HouseType::SmallBrickHouse,
            HouseType::SmallPlasterHouse,
            HouseType::SmallFieldStoneHouse,
            HouseType::SmallWoodHouse,
            HouseType::SmallWoodAndPlasterHouse,
            HouseType::SmallThatchedRoofCottage,
            HouseType::TwoStoryWoodAndPlasterHouse,
            HouseType::TwoStoryStoneAndPlasterHouse,
            HouseType::LargeBrickHouse,
            HouseType::LargePatioHouse,
            HouseType::LargeMarbleHouse,
            HouseType::Tower,
            HouseType::SmallStoneTower,
            HouseType::Castle,
            HouseType::Keep,
        ];
        for t in types {
            let def = HouseDefinition::for_type(t);
            assert_eq!(def.house_type, t);
            assert!(def.cost > 0);
        }
    }

    #[test]
    fn definition_values_are_correct_for_castle() {
        let def = HouseDefinition::for_type(HouseType::Castle);
        assert_eq!(def.name, "Castle");
        assert_eq!(def.max_lockdowns, 56);
        assert_eq!(def.max_secures, 16);
        assert_eq!(def.cost, 865_250);
        assert_eq!(def.stories, 3);
    }

    // ---- creation tests ----

    #[test]
    fn new_house_has_expected_defaults() {
        let h = sample_house();
        assert_eq!(h.serial, 1);
        assert_eq!(h.owner, 100);
        assert_eq!(h.security, HouseSecurity::Private);
        assert_eq!(h.decay_state, HouseDecayState::LikeNew);
        assert!(h.co_owners.is_empty());
        assert!(h.friends.is_empty());
        assert!(h.bans.is_empty());
        assert!(h.locked_downs.is_empty());
        assert!(h.secures.is_empty());
    }

    // ---- co-owner tests ----

    #[test]
    fn add_and_remove_co_owner() {
        let mut h = sample_house();
        assert!(h.add_co_owner(200));
        assert!(h.co_owners.contains(&200));
        assert!(h.remove_co_owner(200));
        assert!(!h.co_owners.contains(&200));
    }

    #[test]
    fn cannot_add_owner_as_co_owner() {
        let mut h = sample_house();
        assert!(!h.add_co_owner(100));
    }

    #[test]
    fn adding_co_owner_removes_from_friends_and_bans() {
        let mut h = sample_house();
        h.friends.insert(200);
        h.bans.insert(200);
        h.add_co_owner(200);
        assert!(!h.friends.contains(&200));
        assert!(!h.bans.contains(&200));
        assert!(h.co_owners.contains(&200));
    }

    // ---- friend tests ----

    #[test]
    fn add_and_remove_friend() {
        let mut h = sample_house();
        assert!(h.add_friend(300));
        assert!(h.friends.contains(&300));
        assert!(h.remove_friend(300));
        assert!(!h.friends.contains(&300));
    }

    #[test]
    fn cannot_add_owner_or_co_owner_as_friend() {
        let mut h = sample_house();
        assert!(!h.add_friend(100)); // owner
        h.add_co_owner(200);
        assert!(!h.add_friend(200)); // co-owner
    }

    #[test]
    fn adding_friend_removes_from_bans() {
        let mut h = sample_house();
        h.bans.insert(300);
        h.add_friend(300);
        assert!(!h.bans.contains(&300));
        assert!(h.friends.contains(&300));
    }

    // ---- ban tests ----

    #[test]
    fn ban_and_unban() {
        let mut h = sample_house();
        assert!(h.ban(400));
        assert!(h.bans.contains(&400));
        assert!(h.unban(400));
        assert!(!h.bans.contains(&400));
    }

    #[test]
    fn cannot_ban_owner_or_co_owner() {
        let mut h = sample_house();
        assert!(!h.ban(100)); // owner
        h.add_co_owner(200);
        assert!(!h.ban(200)); // co-owner
    }

    #[test]
    fn banning_removes_from_friends() {
        let mut h = sample_house();
        h.friends.insert(400);
        h.ban(400);
        assert!(!h.friends.contains(&400));
        assert!(h.bans.contains(&400));
    }

    // ---- access tests ----

    #[test]
    fn owner_always_has_access() {
        let mut h = sample_house();
        h.security = HouseSecurity::OwnerOnly;
        assert!(h.has_access(100, None, None));
    }

    #[test]
    fn banned_character_has_no_access() {
        let mut h = sample_house();
        h.security = HouseSecurity::Public;
        h.ban(500);
        assert!(!h.has_access(500, None, None));
    }

    #[test]
    fn public_house_grants_access_to_strangers() {
        let mut h = sample_house();
        h.security = HouseSecurity::Public;
        assert!(h.has_access(999, None, None));
    }

    #[test]
    fn private_house_denies_strangers() {
        let h = sample_house(); // default is Private
        assert!(!h.has_access(999, None, None));
    }

    #[test]
    fn guild_only_allows_same_guild() {
        let mut h = sample_house();
        h.security = HouseSecurity::GuildOnly;
        assert!(h.has_access(999, Some(1), Some(1)));
        assert!(!h.has_access(999, Some(2), Some(1)));
        assert!(!h.has_access(999, None, Some(1)));
    }

    #[test]
    fn friends_have_access_in_private_mode() {
        let mut h = sample_house();
        h.add_friend(300);
        assert!(h.has_access(300, None, None));
    }

    #[test]
    fn co_owners_have_access_in_owner_only_mode() {
        let mut h = sample_house();
        h.security = HouseSecurity::OwnerOnly;
        h.add_co_owner(200);
        assert!(h.has_access(200, None, None));
    }

    // ---- lock-down tests ----

    #[test]
    fn lock_down_and_release_item() {
        let mut h = sample_house();
        assert!(h.lock_down_item(5001));
        assert!(h.locked_downs.contains(&5001));
        assert!(h.release_item(5001));
        assert!(!h.locked_downs.contains(&5001));
    }

    #[test]
    fn lock_down_respects_limit() {
        let mut h = sample_house();
        let def = HouseDefinition::for_type(h.house_type);
        for i in 0..def.max_lockdowns {
            assert!(h.lock_down_item(6000 + i));
        }
        assert!(!h.lock_down_item(9999));
    }

    #[test]
    fn duplicate_lock_down_returns_false() {
        let mut h = sample_house();
        assert!(h.lock_down_item(5001));
        assert!(!h.lock_down_item(5001));
    }

    // ---- secure tests ----

    #[test]
    fn secure_and_release_container() {
        let mut h = sample_house();
        assert!(h.secure_container(7001));
        assert!(h.secures.contains(&7001));
        assert!(h.release_container(7001));
        assert!(!h.secures.contains(&7001));
    }

    #[test]
    fn secure_respects_limit() {
        let mut h = sample_house();
        let def = HouseDefinition::for_type(h.house_type);
        for i in 0..def.max_secures {
            assert!(h.secure_container(8000 + i));
        }
        assert!(!h.secure_container(9999));
    }

    // ---- decay / refresh tests ----

    #[test]
    fn new_house_decay_level_is_like_new() {
        let h = sample_house();
        assert_eq!(h.decay_level(), HouseDecayState::LikeNew);
    }

    #[test]
    fn refresh_resets_decay_state() {
        let mut h = sample_house();
        h.decay_state = HouseDecayState::Condemned;
        h.refresh();
        assert_eq!(h.decay_state, HouseDecayState::LikeNew);
    }

    #[test]
    fn decay_level_progresses_with_time() {
        let mut h = sample_house();
        // Simulate 3 days elapsed
        h.last_refresh = SystemTime::now() - Duration::from_secs(3 * 86_400);
        assert_eq!(h.decay_level(), HouseDecayState::FairlyWorn);
    }

    #[test]
    fn decay_level_condemned_after_six_days() {
        let mut h = sample_house();
        h.last_refresh = SystemTime::now() - Duration::from_secs(6 * 86_400);
        assert_eq!(h.decay_level(), HouseDecayState::Condemned);
    }

    // ---- 15 definitions count ----

    #[test]
    fn exactly_fifteen_definitions_exist() {
        assert_eq!(HOUSE_DEFINITIONS.len(), 15);
    }
}
