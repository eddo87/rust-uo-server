use byteorder::{BigEndian, WriteBytesExt};

// ---------------------------------------------------------------------------
// 0x11 - Status Bar Info (variable length)
// ---------------------------------------------------------------------------

/// Tier of extended stats sent in a status bar packet (type_flag field).
///   0 = basic stats only (no extended block)
///   1 = T2A era stats
///   2 = unused (placeholder)
///   3 = Renaissance era stats (adds stat-locks)
///   4 = AOS era stats (adds damage/resistances)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusTypeFlag {
    Basic = 0,
    T2A = 1,
    Renaissance = 3,
    AOS = 4,
}

/// All data needed to serialise a 0x11 Status Bar Info packet.
#[derive(Debug, Clone)]
pub struct StatBarData {
    pub serial: u32,
    pub name: String,
    pub hit_points: u16,
    pub max_hit_points: u16,
    pub name_change_flag: bool,
    /// 0 = male human, 1 = female human, 2 = male elf, etc.
    pub body_type: u8,
    pub str_stat: u16,
    pub dex_stat: u16,
    pub int_stat: u16,
    pub stamina: u16,
    pub max_stamina: u16,
    pub mana: u16,
    pub max_mana: u16,
    pub gold: u32,
    /// Physical resist (in percent) for client display
    pub armor_rating: u16,
    pub weight: u16,

    // --- T2A (type_flag >= 1) ---
    pub max_weight: u16,
    /// 1=Human, 2=Elf, 3=Gargoyle
    pub race: u8,

    // --- Renaissance (type_flag >= 3) ---
    pub stat_cap: u16,
    pub followers: u8,
    pub max_followers: u8,

    // --- AOS (type_flag >= 4) ---
    pub fire_resist: u16,
    pub cold_resist: u16,
    pub poison_resist: u16,
    pub energy_resist: u16,
    pub luck: u16,
    pub damage_min: u16,
    pub damage_max: u16,
    pub tithing_points: u32,
}

impl Default for StatBarData {
    fn default() -> Self {
        Self {
            serial: 0,
            name: String::new(),
            hit_points: 0,
            max_hit_points: 0,
            name_change_flag: false,
            body_type: 0,
            str_stat: 0,
            dex_stat: 0,
            int_stat: 0,
            stamina: 0,
            max_stamina: 0,
            mana: 0,
            max_mana: 0,
            gold: 0,
            armor_rating: 0,
            weight: 0,
            max_weight: 0,
            race: 1,
            stat_cap: 225,
            followers: 0,
            max_followers: 5,
            fire_resist: 0,
            cold_resist: 0,
            poison_resist: 0,
            energy_resist: 0,
            luck: 0,
            damage_min: 0,
            damage_max: 0,
            tithing_points: 0,
        }
    }
}

/// Build a 0x11 Status Bar Info packet.
///
/// The packet layout is variable-length.  The `type_flag` controls how many
/// extended stat tiers are appended after the basic block.
pub fn build_status_bar_packet(data: &StatBarData, type_flag: StatusTypeFlag) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();

    buf.push(0x11); // packet id

    // Placeholder for length (u16) - we will patch it at the end.
    buf.write_u16::<BigEndian>(0).unwrap();

    buf.write_u32::<BigEndian>(data.serial).unwrap();

    // Name – 30 bytes, null-padded
    let mut name_bytes = [0u8; 30];
    let name_src = data.name.as_bytes();
    let copy_len = name_src.len().min(30);
    name_bytes[..copy_len].copy_from_slice(&name_src[..copy_len]);
    buf.extend_from_slice(&name_bytes);

    buf.write_u16::<BigEndian>(data.hit_points).unwrap();
    buf.write_u16::<BigEndian>(data.max_hit_points).unwrap();

    buf.push(if data.name_change_flag { 0xFF } else { 0x00 });

    buf.push(type_flag as u8);

    buf.push(data.body_type);

    buf.write_u16::<BigEndian>(data.str_stat).unwrap();
    buf.write_u16::<BigEndian>(data.dex_stat).unwrap();
    buf.write_u16::<BigEndian>(data.int_stat).unwrap();
    buf.write_u16::<BigEndian>(data.stamina).unwrap();
    buf.write_u16::<BigEndian>(data.max_stamina).unwrap();
    buf.write_u16::<BigEndian>(data.mana).unwrap();
    buf.write_u16::<BigEndian>(data.max_mana).unwrap();
    buf.write_u32::<BigEndian>(data.gold).unwrap();
    buf.write_u16::<BigEndian>(data.armor_rating).unwrap();
    buf.write_u16::<BigEndian>(data.weight).unwrap();

    // Extended tiers ---------------------------------------------------------
    if type_flag as u8 >= StatusTypeFlag::T2A as u8 {
        buf.write_u16::<BigEndian>(data.max_weight).unwrap();
        buf.push(data.race);
    }

    if type_flag as u8 >= StatusTypeFlag::Renaissance as u8 {
        buf.write_u16::<BigEndian>(data.stat_cap).unwrap();
        buf.push(data.followers);
        buf.push(data.max_followers);
    }

    if type_flag as u8 >= StatusTypeFlag::AOS as u8 {
        buf.write_u16::<BigEndian>(data.fire_resist).unwrap();
        buf.write_u16::<BigEndian>(data.cold_resist).unwrap();
        buf.write_u16::<BigEndian>(data.poison_resist).unwrap();
        buf.write_u16::<BigEndian>(data.energy_resist).unwrap();
        buf.write_u16::<BigEndian>(data.luck).unwrap();
        buf.write_u16::<BigEndian>(data.damage_min).unwrap();
        buf.write_u16::<BigEndian>(data.damage_max).unwrap();
        buf.write_u32::<BigEndian>(data.tithing_points).unwrap();
    }

    // Patch the length field (bytes 1..3)
    let total_len = buf.len() as u16;
    buf[1] = (total_len >> 8) as u8;
    buf[2] = (total_len & 0xFF) as u8;

    buf
}

// ---------------------------------------------------------------------------
// 0x3A - Skill Update (variable length)
// ---------------------------------------------------------------------------

/// Lock state for a skill or stat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillLock {
    Up = 0,
    Down = 1,
    Locked = 2,
}

impl SkillLock {
    pub fn from_u8(val: u8) -> Option<SkillLock> {
        match val {
            0 => Some(SkillLock::Up),
            1 => Some(SkillLock::Down),
            2 => Some(SkillLock::Locked),
            _ => None,
        }
    }
}

/// Represents one skill row inside a 0x3A packet.
#[derive(Debug, Clone)]
pub struct SkillEntry {
    /// 1-based skill id
    pub skill_id: u16,
    /// Current value * 10  (e.g. 100.0 = 1000)
    pub value: u16,
    /// Raw / unmodified value * 10
    pub base_value: u16,
    pub lock: SkillLock,
    /// Skill cap * 10 (only used with `with_caps` list type)
    pub cap: u16,
}

/// Type byte that opens the skill update body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillListType {
    /// 0x00 - Full list, no caps
    FullList = 0x00,
    /// 0x02 - Full list, with caps
    FullListWithCaps = 0x02,
    /// 0xDF - Single skill update, no cap
    SingleUpdate = 0xDF,
    /// 0xDF + cap - Single skill update, with cap
    SingleUpdateWithCap = 0xFF,
}

/// All data needed to serialise a 0x3A Skill Update packet.
#[derive(Debug, Clone)]
pub struct SkillUpdateData {
    pub list_type: SkillListType,
    pub skills: Vec<SkillEntry>,
}

/// Build a 0x3A Skill Update packet.
pub fn build_skill_update_packet(data: &SkillUpdateData) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();

    buf.push(0x3A); // packet id

    // Placeholder for length (u16)
    buf.write_u16::<BigEndian>(0).unwrap();

    buf.push(data.list_type as u8);

    let with_caps = data.list_type == SkillListType::FullListWithCaps
        || data.list_type == SkillListType::SingleUpdateWithCap;

    for skill in &data.skills {
        buf.write_u16::<BigEndian>(skill.skill_id).unwrap();
        buf.write_u16::<BigEndian>(skill.value).unwrap();
        buf.write_u16::<BigEndian>(skill.base_value).unwrap();
        buf.push(skill.lock as u8);
        if with_caps {
            buf.write_u16::<BigEndian>(skill.cap).unwrap();
        }
    }

    // Full-list variants are terminated with a 0x0000 skill id sentinel.
    if data.list_type == SkillListType::FullList
        || data.list_type == SkillListType::FullListWithCaps
    {
        buf.write_u16::<BigEndian>(0x0000).unwrap();
    }

    // Patch length
    let total_len = buf.len() as u16;
    buf[1] = (total_len >> 8) as u8;
    buf[2] = (total_len & 0xFF) as u8;

    buf
}

// ---------------------------------------------------------------------------
// Client -> Server: Skill Lock Change request (0x3A, 6 bytes)
// ---------------------------------------------------------------------------

/// Result of parsing a client-sent skill lock change (0x3A).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillLockChange {
    pub skill_id: u16,
    pub lock: SkillLock,
}

/// Parse a client skill-lock change request.
///
/// The client sends: `3A 00 06 <skill_id:u16> <lock:u8>`
/// `data` should be the full packet bytes starting at the 0x3A id byte.
///
/// Returns `None` when the packet is malformed.
pub fn parse_skill_lock_change(data: &[u8]) -> Option<SkillLockChange> {
    // Minimum packet: id(1) + len(2) + skill_id(2) + lock(1) = 6
    if data.len() < 6 {
        return None;
    }
    if data[0] != 0x3A {
        return None;
    }

    let skill_id = ((data[3] as u16) << 8) | (data[4] as u16);
    let lock = SkillLock::from_u8(data[5])?;

    Some(SkillLockChange { skill_id, lock })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Status Bar (0x11) tests ------------------------------------------

    fn sample_stat_bar() -> StatBarData {
        StatBarData {
            serial: 0x00_00_00_01,
            name: "TestChar".into(),
            hit_points: 100,
            max_hit_points: 125,
            name_change_flag: false,
            body_type: 0x01,
            str_stat: 50,
            dex_stat: 40,
            int_stat: 30,
            stamina: 40,
            max_stamina: 40,
            mana: 30,
            max_mana: 30,
            gold: 1000,
            armor_rating: 15,
            weight: 80,
            max_weight: 300,
            race: 1,
            stat_cap: 225,
            followers: 1,
            max_followers: 5,
            fire_resist: 10,
            cold_resist: 5,
            poison_resist: 8,
            energy_resist: 12,
            luck: 100,
            damage_min: 5,
            damage_max: 15,
            tithing_points: 0,
        }
    }

    #[test]
    fn status_bar_basic_packet_starts_with_0x11() {
        let data = sample_stat_bar();
        let pkt = build_status_bar_packet(&data, StatusTypeFlag::Basic);
        assert_eq!(pkt[0], 0x11);
    }

    #[test]
    fn status_bar_basic_length_field_matches() {
        let data = sample_stat_bar();
        let pkt = build_status_bar_packet(&data, StatusTypeFlag::Basic);
        let encoded_len = ((pkt[1] as u16) << 8) | (pkt[2] as u16);
        assert_eq!(encoded_len as usize, pkt.len());
    }

    #[test]
    fn status_bar_basic_contains_serial() {
        let data = sample_stat_bar();
        let pkt = build_status_bar_packet(&data, StatusTypeFlag::Basic);
        // serial is at bytes 3..7
        assert_eq!(&pkt[3..7], &[0x00, 0x00, 0x00, 0x01]);
    }

    #[test]
    fn status_bar_basic_contains_name() {
        let data = sample_stat_bar();
        let pkt = build_status_bar_packet(&data, StatusTypeFlag::Basic);
        // name starts at byte 7, 30 bytes
        let name_bytes = &pkt[7..37];
        assert_eq!(&name_bytes[..8], b"TestChar");
        // remaining bytes should be null
        assert!(name_bytes[8..].iter().all(|&b| b == 0));
    }

    #[test]
    fn status_bar_basic_type_flag_is_zero() {
        let data = sample_stat_bar();
        let pkt = build_status_bar_packet(&data, StatusTypeFlag::Basic);
        // type_flag is after: id(1)+len(2)+serial(4)+name(30)+hp(2)+maxhp(2)+namechange(1) = 42
        assert_eq!(pkt[42], 0x00);
    }

    #[test]
    fn status_bar_t2a_is_longer_than_basic() {
        let data = sample_stat_bar();
        let basic = build_status_bar_packet(&data, StatusTypeFlag::Basic);
        let t2a = build_status_bar_packet(&data, StatusTypeFlag::T2A);
        // T2A adds max_weight(2) + race(1) = 3 bytes
        assert_eq!(t2a.len(), basic.len() + 3);
    }

    #[test]
    fn status_bar_renaissance_is_longer_than_t2a() {
        let data = sample_stat_bar();
        let t2a = build_status_bar_packet(&data, StatusTypeFlag::T2A);
        let ren = build_status_bar_packet(&data, StatusTypeFlag::Renaissance);
        // Renaissance adds stat_cap(2) + followers(1) + max_followers(1) = 4 bytes
        assert_eq!(ren.len(), t2a.len() + 4);
    }

    #[test]
    fn status_bar_aos_is_longer_than_renaissance() {
        let data = sample_stat_bar();
        let ren = build_status_bar_packet(&data, StatusTypeFlag::Renaissance);
        let aos = build_status_bar_packet(&data, StatusTypeFlag::AOS);
        // AOS adds fire(2)+cold(2)+poison(2)+energy(2)+luck(2)+dmgmin(2)+dmgmax(2)+tithe(4) = 18
        assert_eq!(aos.len(), ren.len() + 18);
    }

    #[test]
    fn status_bar_aos_length_field_matches() {
        let data = sample_stat_bar();
        let pkt = build_status_bar_packet(&data, StatusTypeFlag::AOS);
        let encoded_len = ((pkt[1] as u16) << 8) | (pkt[2] as u16);
        assert_eq!(encoded_len as usize, pkt.len());
    }

    #[test]
    fn status_bar_name_truncated_at_30() {
        let mut data = sample_stat_bar();
        data.name = "A".repeat(50);
        let pkt = build_status_bar_packet(&data, StatusTypeFlag::Basic);
        let name_bytes = &pkt[7..37];
        assert!(name_bytes.iter().all(|&b| b == b'A'));
    }

    // ---- Skill Update (0x3A) tests ----------------------------------------

    fn sample_skills() -> Vec<SkillEntry> {
        vec![
            SkillEntry {
                skill_id: 1,
                value: 1000,
                base_value: 1000,
                lock: SkillLock::Up,
                cap: 1200,
            },
            SkillEntry {
                skill_id: 2,
                value: 500,
                base_value: 500,
                lock: SkillLock::Locked,
                cap: 1000,
            },
        ]
    }

    #[test]
    fn skill_full_list_starts_with_0x3a() {
        let data = SkillUpdateData {
            list_type: SkillListType::FullList,
            skills: sample_skills(),
        };
        let pkt = build_skill_update_packet(&data);
        assert_eq!(pkt[0], 0x3A);
    }

    #[test]
    fn skill_full_list_length_field_matches() {
        let data = SkillUpdateData {
            list_type: SkillListType::FullList,
            skills: sample_skills(),
        };
        let pkt = build_skill_update_packet(&data);
        let encoded_len = ((pkt[1] as u16) << 8) | (pkt[2] as u16);
        assert_eq!(encoded_len as usize, pkt.len());
    }

    #[test]
    fn skill_full_list_has_type_byte() {
        let data = SkillUpdateData {
            list_type: SkillListType::FullList,
            skills: sample_skills(),
        };
        let pkt = build_skill_update_packet(&data);
        assert_eq!(pkt[3], 0x00);
    }

    #[test]
    fn skill_full_list_ends_with_sentinel() {
        let data = SkillUpdateData {
            list_type: SkillListType::FullList,
            skills: sample_skills(),
        };
        let pkt = build_skill_update_packet(&data);
        let len = pkt.len();
        // Last two bytes should be 0x00 0x00 (sentinel)
        assert_eq!(&pkt[len - 2..], &[0x00, 0x00]);
    }

    #[test]
    fn skill_full_list_no_caps_entry_size_is_7() {
        // Each entry without caps: skill_id(2) + value(2) + base(2) + lock(1) = 7
        let data = SkillUpdateData {
            list_type: SkillListType::FullList,
            skills: sample_skills(),
        };
        let pkt = build_skill_update_packet(&data);
        // Total: id(1) + len(2) + type(1) + 2*7 + sentinel(2) = 20
        assert_eq!(pkt.len(), 20);
    }

    #[test]
    fn skill_full_list_with_caps_entry_size_is_9() {
        // Each entry with caps: skill_id(2) + value(2) + base(2) + lock(1) + cap(2) = 9
        let data = SkillUpdateData {
            list_type: SkillListType::FullListWithCaps,
            skills: sample_skills(),
        };
        let pkt = build_skill_update_packet(&data);
        // Total: id(1) + len(2) + type(1) + 2*9 + sentinel(2) = 24
        assert_eq!(pkt.len(), 24);
    }

    #[test]
    fn skill_full_list_with_caps_type_byte() {
        let data = SkillUpdateData {
            list_type: SkillListType::FullListWithCaps,
            skills: sample_skills(),
        };
        let pkt = build_skill_update_packet(&data);
        assert_eq!(pkt[3], 0x02);
    }

    #[test]
    fn skill_single_update_no_sentinel() {
        let data = SkillUpdateData {
            list_type: SkillListType::SingleUpdate,
            skills: vec![sample_skills()[0].clone()],
        };
        let pkt = build_skill_update_packet(&data);
        // id(1) + len(2) + type(1) + 1*7 = 11, no sentinel
        assert_eq!(pkt.len(), 11);
    }

    #[test]
    fn skill_single_update_with_cap() {
        let data = SkillUpdateData {
            list_type: SkillListType::SingleUpdateWithCap,
            skills: vec![sample_skills()[0].clone()],
        };
        let pkt = build_skill_update_packet(&data);
        // id(1) + len(2) + type(1) + 1*9 = 13, no sentinel
        assert_eq!(pkt.len(), 13);
    }

    #[test]
    fn skill_first_entry_values_correct() {
        let data = SkillUpdateData {
            list_type: SkillListType::FullList,
            skills: sample_skills(),
        };
        let pkt = build_skill_update_packet(&data);
        // First entry starts at byte 4
        // skill_id = 1  -> 0x00 0x01
        assert_eq!(&pkt[4..6], &[0x00, 0x01]);
        // value = 1000  -> 0x03 0xE8
        assert_eq!(&pkt[6..8], &[0x03, 0xE8]);
        // base = 1000   -> 0x03 0xE8
        assert_eq!(&pkt[8..10], &[0x03, 0xE8]);
        // lock = Up(0)
        assert_eq!(pkt[10], 0x00);
    }

    // ---- Skill Lock Change parse tests ------------------------------------

    #[test]
    fn parse_valid_skill_lock_change() {
        let data: Vec<u8> = vec![0x3A, 0x00, 0x06, 0x00, 0x05, 0x02];
        let result = parse_skill_lock_change(&data);
        assert!(result.is_some());
        let change = result.unwrap();
        assert_eq!(change.skill_id, 5);
        assert_eq!(change.lock, SkillLock::Locked);
    }

    #[test]
    fn parse_skill_lock_change_lock_up() {
        let data: Vec<u8> = vec![0x3A, 0x00, 0x06, 0x00, 0x01, 0x00];
        let change = parse_skill_lock_change(&data).unwrap();
        assert_eq!(change.lock, SkillLock::Up);
    }

    #[test]
    fn parse_skill_lock_change_lock_down() {
        let data: Vec<u8> = vec![0x3A, 0x00, 0x06, 0x00, 0x01, 0x01];
        let change = parse_skill_lock_change(&data).unwrap();
        assert_eq!(change.lock, SkillLock::Down);
    }

    #[test]
    fn parse_skill_lock_change_too_short() {
        let data: Vec<u8> = vec![0x3A, 0x00, 0x06, 0x00];
        assert!(parse_skill_lock_change(&data).is_none());
    }

    #[test]
    fn parse_skill_lock_change_wrong_id() {
        let data: Vec<u8> = vec![0x11, 0x00, 0x06, 0x00, 0x05, 0x02];
        assert!(parse_skill_lock_change(&data).is_none());
    }

    #[test]
    fn parse_skill_lock_change_invalid_lock() {
        let data: Vec<u8> = vec![0x3A, 0x00, 0x06, 0x00, 0x05, 0x09];
        assert!(parse_skill_lock_change(&data).is_none());
    }

    #[test]
    fn skill_lock_from_u8_roundtrip() {
        assert_eq!(SkillLock::from_u8(0), Some(SkillLock::Up));
        assert_eq!(SkillLock::from_u8(1), Some(SkillLock::Down));
        assert_eq!(SkillLock::from_u8(2), Some(SkillLock::Locked));
        assert_eq!(SkillLock::from_u8(3), None);
    }
}
