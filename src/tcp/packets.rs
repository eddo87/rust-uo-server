use byteorder::{BigEndian, ByteOrder};
use crate::movement::{Direction, MovementRequest, Position, is_running};
use crate::speech::{SpeechRequest, SpeechType};
use crate::state::Shard;

pub fn server_list_packet() -> [u8; 46] {
    let _shards = vec![Shard::new(String::from("My Shard"))];
    let mut buffer: [u8; 46] = [0; 46];
    buffer[0] = 0xA8;
    buffer[1] = 0x00; buffer[2] = 0x2E;
    buffer[3] = 0x00;
    BigEndian::write_u16(&mut buffer[4..6], 1);
    BigEndian::write_u16(&mut buffer[6..8], 0);
    buffer[8..16].copy_from_slice("My Shard".as_bytes());
    buffer[37] = 0x00;
    buffer[38] = 0x00; buffer[39] = 0x00; buffer[40] = 0x00; buffer[41] = 0x00;
    buffer[42] = 0x7F; buffer[43] = 0x00; buffer[44] = 0x00; buffer[45] = 0x01;
    buffer
}

pub fn server_redirect_packet() -> [u8; 11] {
    let mut buffer: [u8; 11] = [0; 11];
    buffer[0] = 0x8C;
    buffer[1] = 0x7F; buffer[2] = 0x00; buffer[3] = 0x00; buffer[4] = 0x01;
    buffer[5] = 0x0A; buffer[6] = 0x21;
    buffer[7] = 0x43; buffer[8] = 0x2F; buffer[9] = 0x3F; buffer[10] = 0xF0;
    buffer
}

pub fn features_packet() -> Vec<u8> { vec![0xB9, 0x00, 0xFF, 0x92, 0xDB] }

pub fn character_list_packet() -> Vec<u8> {
    let mut src = vec![];
    src.push(0xA9); src.append(&mut vec![0x02, 0x08]);
    let character_count: u8 = 0x07;
    src.push(character_count);
    for _ in 0..character_count { src.append(&mut vec![0x00; 60]); }
    src.push(0x01);
    let mut city = vec![];
    city.push(0x00);
    city.append(&mut format!("{:\0<32}", "Britain").as_bytes().into());
    city.append(&mut format!("{:\0<32}", "The Wayfarer's Inn").as_bytes().into());
    let x: u32 = 1602; let y: u32 = 1591; let z: u32 = 20;
    city.append(&mut x.to_be_bytes().into());
    city.append(&mut y.to_be_bytes().into());
    city.append(&mut z.to_be_bytes().into());
    city.append(&mut vec![0x00, 0x00, 0x00, 0x01]);
    let city_description: u32 = 1075074;
    city.append(&mut city_description.to_be_bytes().into());
    city.append(&mut vec![0x00, 0x00, 0x00, 0x00]);
    src.append(&mut city);
    let flags: u32 = 4584;
    src.append(&mut flags.to_be_bytes().into());
    src.append(&mut vec![0xFF, 0xFF]);
    src
}

// -- Movement packet functions --

pub fn parse_movement_request(data: &[u8; 6]) -> Option<MovementRequest> {
    let direction_byte = data[0];
    let direction = Direction::from_byte(direction_byte)?;
    let running = is_running(direction_byte);
    let sequence_number = data[1];
    let fastwalk_key = u32::from_be_bytes([data[2], data[3], data[4], data[5]]);
    Some(MovementRequest { direction, running, sequence_number, fastwalk_key })
}

pub fn movement_ack_packet(sequence: u8, notoriety: u8) -> [u8; 3] { [0x22, sequence, notoriety] }

pub fn movement_reject_packet(sequence: u8, position: Position, direction: Direction) -> [u8; 8] {
    let mut buffer: [u8; 8] = [0; 8];
    buffer[0] = 0x21; buffer[1] = sequence;
    BigEndian::write_u16(&mut buffer[2..4], position.x);
    BigEndian::write_u16(&mut buffer[4..6], position.y);
    buffer[6] = direction.to_byte(); buffer[7] = position.z as u8;
    buffer
}

pub fn draw_player_packet(serial: u32, body_type: u16, hue: u16, status_flags: u8, position: Position, direction: Direction, notoriety: u8) -> [u8; 19] {
    let mut buffer: [u8; 19] = [0; 19];
    buffer[0] = 0x20;
    BigEndian::write_u32(&mut buffer[1..5], serial);
    BigEndian::write_u16(&mut buffer[5..7], body_type);
    buffer[7] = 0x00;
    BigEndian::write_u16(&mut buffer[8..10], hue);
    buffer[10] = status_flags;
    BigEndian::write_u16(&mut buffer[11..13], position.x);
    BigEndian::write_u16(&mut buffer[13..15], position.y);
    BigEndian::write_u16(&mut buffer[15..17], 0x0000);
    buffer[17] = direction.to_byte() | (notoriety & 0x03);
    buffer[18] = position.z as u8;
    buffer
}

// -- Speech packet functions --

pub fn parse_unicode_speech_request(data: &[u8]) -> Option<SpeechRequest> {
    if data.len() < 12 { return None; }
    let _length = BigEndian::read_u16(&data[0..2]);
    let speech_type_byte = data[2];
    let type_value = speech_type_byte & 0x0F;
    let speech_type = SpeechType::from_byte(type_value)?;
    let color = BigEndian::read_u16(&data[3..5]);
    let font = BigEndian::read_u16(&data[5..7]);
    let _language = &data[7..11];
    let text_bytes = &data[11..];
    let text_end = text_bytes.iter().position(|&b| b == 0x00).unwrap_or(text_bytes.len());
    let text = String::from_utf8_lossy(&text_bytes[..text_end]).into_owned();
    Some(SpeechRequest { speech_type, color, font, text })
}

pub fn ascii_speech_packet(serial: u32, model: u16, speech_type: SpeechType, hue: u16, font: u16, name: &str, text: &str) -> Vec<u8> {
    let total_length: u16 = 44 + text.len() as u16 + 1;
    let mut buf = Vec::with_capacity(total_length as usize);
    buf.push(0x1C);
    let mut len_bytes = [0u8; 2]; BigEndian::write_u16(&mut len_bytes, total_length); buf.extend_from_slice(&len_bytes);
    let mut serial_bytes = [0u8; 4]; BigEndian::write_u32(&mut serial_bytes, serial); buf.extend_from_slice(&serial_bytes);
    let mut model_bytes = [0u8; 2]; BigEndian::write_u16(&mut model_bytes, model); buf.extend_from_slice(&model_bytes);
    buf.push(speech_type as u8);
    let mut hue_bytes = [0u8; 2]; BigEndian::write_u16(&mut hue_bytes, hue); buf.extend_from_slice(&hue_bytes);
    let mut font_bytes = [0u8; 2]; BigEndian::write_u16(&mut font_bytes, font); buf.extend_from_slice(&font_bytes);
    let mut name_buf = [0u8; 30];
    let name_bytes = name.as_bytes();
    let name_len = name_bytes.len().min(29);
    name_buf[..name_len].copy_from_slice(&name_bytes[..name_len]);
    buf.extend_from_slice(&name_buf);
    buf.extend_from_slice(text.as_bytes());
    buf.push(0x00);
    buf
}

pub fn unicode_speech_packet(serial: u32, model: u16, speech_type: SpeechType, hue: u16, font: u16, language: &str, name: &str, text: &str) -> Vec<u8> {
    let utf16_chars: Vec<u16> = text.encode_utf16().collect();
    let utf16_byte_len = (utf16_chars.len() + 1) * 2;
    let total_length: u16 = 48 + utf16_byte_len as u16;
    let mut buf = Vec::with_capacity(total_length as usize);
    buf.push(0xAE);
    let mut len_bytes = [0u8; 2]; BigEndian::write_u16(&mut len_bytes, total_length); buf.extend_from_slice(&len_bytes);
    let mut serial_bytes = [0u8; 4]; BigEndian::write_u32(&mut serial_bytes, serial); buf.extend_from_slice(&serial_bytes);
    let mut model_bytes = [0u8; 2]; BigEndian::write_u16(&mut model_bytes, model); buf.extend_from_slice(&model_bytes);
    buf.push(speech_type as u8);
    let mut hue_bytes = [0u8; 2]; BigEndian::write_u16(&mut hue_bytes, hue); buf.extend_from_slice(&hue_bytes);
    let mut font_bytes = [0u8; 2]; BigEndian::write_u16(&mut font_bytes, font); buf.extend_from_slice(&font_bytes);
    let mut lang_buf = [0u8; 4];
    let lang_bytes = language.as_bytes();
    let lang_len = lang_bytes.len().min(3);
    lang_buf[..lang_len].copy_from_slice(&lang_bytes[..lang_len]);
    buf.extend_from_slice(&lang_buf);
    let mut name_buf = [0u8; 30];
    let name_bytes = name.as_bytes();
    let name_len = name_bytes.len().min(29);
    name_buf[..name_len].copy_from_slice(&name_bytes[..name_len]);
    buf.extend_from_slice(&name_buf);
    for ch in &utf16_chars { let mut char_bytes = [0u8; 2]; BigEndian::write_u16(&mut char_bytes, *ch); buf.extend_from_slice(&char_bytes); }
    buf.extend_from_slice(&[0x00, 0x00]);
    buf
}

pub fn system_message_packet(text: &str) -> Vec<u8> {
    ascii_speech_packet(0xFFFFFFFF, 0xFFFF, SpeechType::System, 0x0035, 0x0003, "System", text)
}

// -- In-game init packet functions --

/// 0x1B Login Confirm (37 bytes).
///
/// Sent immediately after character select to tell the client where the
/// player is placed in the world.
pub fn login_confirm_packet(serial: u32, body: u16, x: u16, y: u16, z: i8, dir: u8, map_width: u16, map_height: u16) -> [u8; 37] {
    let mut buf = [0u8; 37];
    buf[0] = 0x1B;
    BigEndian::write_u32(&mut buf[1..5], serial);
    // [5..9] unknown = 0
    BigEndian::write_u16(&mut buf[9..11], body);
    BigEndian::write_u16(&mut buf[11..13], x);
    BigEndian::write_u16(&mut buf[13..15], y);
    // [15..17] unknown = 0
    buf[17] = z as u8;
    buf[18] = dir;
    // [19..25] unknown = 0
    BigEndian::write_u16(&mut buf[25..27], map_width);
    BigEndian::write_u16(&mut buf[27..29], map_height);
    // [29..37] unknown = 0
    buf
}

/// 0x55 Login Complete (1 byte).
pub fn login_complete_packet() -> [u8; 1] {
    [0x55]
}

/// 0x4F Overall Light Level (2 bytes).
pub fn global_light_level_packet(level: u8) -> [u8; 2] {
    [0x4F, level]
}

/// 0x4E Personal Light Level (6 bytes).
pub fn personal_light_level_packet(serial: u32, level: u8) -> [u8; 6] {
    let mut buf = [0u8; 6];
    buf[0] = 0x4E;
    BigEndian::write_u32(&mut buf[1..5], serial);
    buf[5] = level;
    buf
}

/// 0x72 War Mode (5 bytes).
pub fn war_mode_packet(war: bool) -> [u8; 5] {
    [0x72, war as u8, 0x00, 0x32, 0x00]
}

/// Parse a 0x5D Play Character packet body (72 bytes, packet ID already consumed).
///
/// Layout after the packet ID byte:
/// - [0..4]   unknown pattern
/// - [4..34]  character name (30 bytes, null-terminated)
/// - [34..64] unknown
/// - [64..68] character slot (uint32 BE)
/// - [68..72] client IP
///
/// 0x82 Login Deny — sent when login is refused.
///
/// `reason` values: 0x00 = invalid credentials, 0x01 = account in use,
/// 0x02 = account blocked, 0x03 = bad password, 0x04 = idle too long,
/// 0x05 = communication problem, 0x06 = bad communication (client too old).
pub fn login_deny_packet(reason: u8) -> [u8; 2] { [0x82, reason] }

/// Returns `(slot, character_name)` or `None` if the data is too short.
pub fn parse_character_select(data: &[u8]) -> Option<(u32, String)> {
    if data.len() < 68 { return None; }
    let slot = u32::from_be_bytes([data[64], data[65], data[66], data[67]]);
    let name_bytes = &data[4..34];
    let name_end = name_bytes.iter().position(|&b| b == 0).unwrap_or(30);
    let name = String::from_utf8_lossy(&name_bytes[..name_end]).into_owned();
    Some((slot, name))
}

/// Builds a Remove Entity packet (0x1D).
///
/// Sent to clients to remove a mobile or item from their view (e.g. on death).
pub fn remove_entity_packet(serial: u32) -> [u8; 5] {
    let mut buf = [0u8; 5];
    buf[0] = 0x1D;
    BigEndian::write_u32(&mut buf[1..5], serial);
    buf
}

/// Builds a Resurrect packet (0x2C) to restore a ghost to living status.
pub fn resurrect_packet() -> [u8; 2] { [0x2C, 0x01] }

/// Builds a Ghost Mode packet (0x2C) sent when a player dies.
pub fn ghost_mode_packet() -> [u8; 2] { [0x2C, 0x02] }

/// Builds a Damage notification packet (0x0B).
///
/// Sent to inform a client of damage taken.  7 bytes total.
pub fn damage_notification_packet(serial: u32, damage: u16) -> [u8; 7] {
    let mut buf = [0u8; 7];
    buf[0] = 0x0B;
    BigEndian::write_u32(&mut buf[1..5], serial);
    BigEndian::write_u16(&mut buf[5..7], damage);
    buf
}

/// Builds a Mobile Incoming packet (0x78).
///
/// Sent to clients to introduce a new mobile (player or NPC) into their view.
/// The item list is omitted (terminated immediately with 0x00000000).
pub fn mobile_incoming_packet(
    serial: u32,
    body: u16,
    x: u16,
    y: u16,
    z: i8,
    direction: u8,
    hue: u16,
    flags: u8,
    notoriety: u8,
) -> Vec<u8> {
    // Fixed-length portion: 1 (id) + 2 (len) + 4 (serial) + 2 (body) + 2 (x) + 2 (y)
    //   + 1 (z) + 1 (dir) + 2 (hue) + 1 (flags) + 1 (notoriety) + 4 (terminator) = 23 bytes
    let length: u16 = 23;
    let mut buf: Vec<u8> = Vec::with_capacity(length as usize);

    buf.push(0x78);
    buf.push((length >> 8) as u8);
    buf.push((length & 0xFF) as u8);
    buf.extend_from_slice(&serial.to_be_bytes());
    buf.extend_from_slice(&body.to_be_bytes());
    buf.extend_from_slice(&x.to_be_bytes());
    buf.extend_from_slice(&y.to_be_bytes());
    buf.push(z as u8);
    buf.push(direction);
    buf.extend_from_slice(&hue.to_be_bytes());
    buf.push(flags);
    buf.push(notoriety);
    buf.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

    buf
}

/// Builds a Mobile Update packet (0x20).
///
/// Sent to update a mobile's body, position, direction, and hue.
pub fn mobile_update_packet(
    serial: u32,
    body: u16,
    x: u16,
    y: u16,
    z: i8,
    direction: u8,
    hue: u16,
    flags: u8,
) -> [u8; 19] {
    let mut buf = [0u8; 19];
    buf[0] = 0x20;
    BigEndian::write_u32(&mut buf[1..5], serial);
    BigEndian::write_u16(&mut buf[5..7], body);
    buf[7] = 0x00;
    BigEndian::write_u16(&mut buf[8..10], hue);
    buf[10] = flags;
    BigEndian::write_u16(&mut buf[11..13], x);
    BigEndian::write_u16(&mut buf[13..15], y);
    buf[15] = 0x00;
    buf[16] = direction;
    buf[17] = z as u8;
    buf[18] = 0x00;
    buf
}

/// Parses an incoming speech request packet (0xAD) from raw bytes.
///
/// Returns `(type, hue, font, text)` on success, or `None` if the
/// buffer is too short or malformed.
pub fn parse_speech_request(data: &[u8]) -> Option<(u8, u16, u16, String)> {
    if data.len() < 13 {
        return None;
    }
    if data[0] != 0xAD {
        return None;
    }
    let speech_type = data[3];
    let hue = BigEndian::read_u16(&data[4..6]);
    let font = BigEndian::read_u16(&data[6..8]);
    let text_start = 12;
    if data.len() <= text_start {
        return None;
    }
    let text_bytes = &data[text_start..];
    let null_pos = text_bytes.iter().position(|&b| b == 0).unwrap_or(text_bytes.len());
    let text = String::from_utf8_lossy(&text_bytes[..null_pos]).into_owned();
    Some((speech_type, hue, font, text))
}

/// Builds an ASCII speech packet (0x1C) directed at a mobile.
pub fn player_speech_packet(
    serial: u32,
    body: u16,
    speech_type: u8,
    hue: u16,
    font: u16,
    name: &str,
    text: &str,
) -> Vec<u8> {
    let text_len = text.len();
    let length: u16 = (44 + text_len + 1) as u16;
    let mut buf: Vec<u8> = Vec::with_capacity(length as usize);
    buf.push(0x1C);
    buf.extend_from_slice(&length.to_be_bytes());
    buf.extend_from_slice(&serial.to_be_bytes());
    buf.extend_from_slice(&body.to_be_bytes());
    buf.push(speech_type);
    buf.extend_from_slice(&hue.to_be_bytes());
    buf.extend_from_slice(&font.to_be_bytes());
    let name_bytes = name.as_bytes();
    let name_len = name_bytes.len().min(29);
    buf.extend_from_slice(&name_bytes[..name_len]);
    for _ in name_len..30 {
        buf.push(0x00);
    }
    buf.extend_from_slice(text.as_bytes());
    buf.push(0x00);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_creates_the_correct_packet() {
        let packet = character_list_packet();
        assert_eq!(packet[0], 0xA9);
        assert_eq!(packet.len(), 520);
    }

    #[test]
    fn parse_movement_request_walking_north() {
        let data: [u8; 6] = [0x00, 0x01, 0x00, 0x00, 0x00, 0x00];
        let req = parse_movement_request(&data).unwrap();
        assert_eq!(req.direction, Direction::North);
        assert!(!req.running);
    }

    #[test]
    fn movement_ack_packet_format() {
        let packet = movement_ack_packet(0x05, 0x01);
        assert_eq!(packet[0], 0x22);
        assert_eq!(packet.len(), 3);
    }

    #[test]
    fn parse_unicode_speech_request_regular() {
        let text = "Hello";
        let mut data: Vec<u8> = Vec::new();
        let total_len: u16 = 1 + 2 + 1 + 2 + 2 + 4 + text.len() as u16 + 1;
        data.extend_from_slice(&total_len.to_be_bytes());
        data.push(0x00);
        data.extend_from_slice(&0x0035u16.to_be_bytes());
        data.extend_from_slice(&0x0003u16.to_be_bytes());
        data.extend_from_slice(b"ENU\0");
        data.extend_from_slice(text.as_bytes());
        data.push(0x00);
        let result = parse_unicode_speech_request(&data).unwrap();
        assert_eq!(result.speech_type, SpeechType::Regular);
        assert_eq!(result.text, "Hello");
    }

    #[test]
    fn system_message_packet_structure() {
        let packet = system_message_packet("Server is restarting");
        assert_eq!(packet[0], 0x1C);
        assert_eq!(BigEndian::read_u32(&packet[3..7]), 0xFFFFFFFF);
    }

    #[test]
    fn login_confirm_packet_structure() {
        let pkt = login_confirm_packet(0x00000001, 0x0190, 1496, 1628, 10, 0x00, 6144, 4096);
        assert_eq!(pkt.len(), 37);
        assert_eq!(pkt[0], 0x1B);
        assert_eq!(BigEndian::read_u32(&pkt[1..5]), 0x00000001);
        assert_eq!(BigEndian::read_u16(&pkt[9..11]), 0x0190);
        assert_eq!(BigEndian::read_u16(&pkt[11..13]), 1496);
        assert_eq!(BigEndian::read_u16(&pkt[13..15]), 1628);
        assert_eq!(pkt[17], 10u8);
        assert_eq!(BigEndian::read_u16(&pkt[25..27]), 6144);
        assert_eq!(BigEndian::read_u16(&pkt[27..29]), 4096);
    }

    #[test]
    fn login_complete_packet_structure() {
        let pkt = login_complete_packet();
        assert_eq!(pkt, [0x55]);
    }

    #[test]
    fn global_light_level_packet_structure() {
        let pkt = global_light_level_packet(15);
        assert_eq!(pkt, [0x4F, 15]);
    }

    #[test]
    fn personal_light_level_packet_structure() {
        let pkt = personal_light_level_packet(0xDEADBEEF, 20);
        assert_eq!(pkt.len(), 6);
        assert_eq!(pkt[0], 0x4E);
        assert_eq!(BigEndian::read_u32(&pkt[1..5]), 0xDEADBEEF);
        assert_eq!(pkt[5], 20);
    }

    #[test]
    fn war_mode_packet_structure() {
        let peace = war_mode_packet(false);
        assert_eq!(peace, [0x72, 0x00, 0x00, 0x32, 0x00]);
        let war = war_mode_packet(true);
        assert_eq!(war[0], 0x72);
        assert_eq!(war[1], 0x01);
    }

    #[test]
    fn login_deny_packet_structure() {
        let pkt = login_deny_packet(0x06);
        assert_eq!(pkt, [0x82, 0x06]);
    }

    #[test]
    fn parse_character_select_valid() {
        let mut data = vec![0u8; 72];
        // slot = 2 at offset 64
        data[64] = 0x00; data[65] = 0x00; data[66] = 0x00; data[67] = 0x02;
        // name = "TestChar" at offset 4
        let name = b"TestChar";
        data[4..4 + name.len()].copy_from_slice(name);
        let result = parse_character_select(&data);
        assert!(result.is_some());
        let (slot, name) = result.unwrap();
        assert_eq!(slot, 2);
        assert_eq!(name, "TestChar");
    }

    #[test]
    fn parse_character_select_too_short() {
        let data = vec![0u8; 60];
        assert!(parse_character_select(&data).is_none());
    }

    #[test]
    fn mobile_incoming_packet_has_correct_structure() {
        let packet = mobile_incoming_packet(0x0000_0001, 0x0190, 100, 200, 10, 0x00, 0x0000, 0x00, 0x03);
        assert_eq!(packet[0], 0x78);
        assert_eq!(packet[1], 0x00); assert_eq!(packet[2], 0x17);
        assert_eq!(&packet[3..7], &[0x00, 0x00, 0x00, 0x01]);
        assert_eq!(&packet[7..9], &[0x01, 0x90]);
        assert_eq!(&packet[9..11], &[0x00, 0x64]);
        assert_eq!(&packet[11..13], &[0x00, 0xC8]);
        assert_eq!(packet[13], 0x0A);
        assert_eq!(packet[14], 0x00);
        assert_eq!(&packet[15..17], &[0x00, 0x00]);
        assert_eq!(packet[17], 0x00);
        assert_eq!(packet[18], 0x03);
        assert_eq!(&packet[19..23], &[0x00, 0x00, 0x00, 0x00]);
        assert_eq!(packet.len(), 23);
    }

    #[test]
    fn mobile_update_packet_has_correct_structure() {
        let packet = mobile_update_packet(0x0000_0002, 0x0191, 150, 250, -5, 0x02, 0x0835, 0x00);
        assert_eq!(packet.len(), 19);
        assert_eq!(packet[0], 0x20);
        assert_eq!(&packet[1..5], &[0x00, 0x00, 0x00, 0x02]);
        assert_eq!(&packet[5..7], &[0x01, 0x91]);
        assert_eq!(packet[7], 0x00);
        assert_eq!(&packet[8..10], &[0x08, 0x35]);
        assert_eq!(packet[10], 0x00);
        assert_eq!(&packet[11..13], &[0x00, 0x96]);
        assert_eq!(&packet[13..15], &[0x00, 0xFA]);
        assert_eq!(packet[15], 0x00);
        assert_eq!(packet[16], 0x02);
        assert_eq!(packet[17], 251u8);
    }

    #[test]
    fn parse_speech_request_returns_correct_fields() {
        let text = b"Hello\0";
        let length: u16 = (12 + text.len()) as u16;
        let mut data: Vec<u8> = vec![
            0xAD,
            (length >> 8) as u8, (length & 0xFF) as u8,
            0x00,        // type: normal
            0x00, 0x35,  // hue = 53
            0x00, 0x03,  // font = 3
            b'E', b'N', b'U', 0x00,
        ];
        data.extend_from_slice(text);
        let (speech_type, hue, font, msg) = parse_speech_request(&data).unwrap();
        assert_eq!(speech_type, 0x00);
        assert_eq!(hue, 53);
        assert_eq!(font, 3);
        assert_eq!(msg, "Hello");
    }

    #[test]
    fn parse_speech_request_returns_none_for_short_buffer() {
        let data = vec![0xAD, 0x00, 0x05, 0x00];
        assert!(parse_speech_request(&data).is_none());
    }

    #[test]
    fn parse_speech_request_returns_none_for_wrong_id() {
        let data = vec![0x1C, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        assert!(parse_speech_request(&data).is_none());
    }

    #[test]
    fn resurrect_and_ghost_mode_packets() {
        assert_eq!(resurrect_packet(), [0x2C, 0x01]);
        assert_eq!(ghost_mode_packet(), [0x2C, 0x02]);
    }

    #[test]
    fn damage_notification_packet_structure() {
        let pkt = damage_notification_packet(0xDEAD_BEEF, 42);
        assert_eq!(pkt.len(), 7);
        assert_eq!(pkt[0], 0x0B);
        assert_eq!(&pkt[1..5], &[0xDE, 0xAD, 0xBE, 0xEF]);
        assert_eq!(&pkt[5..7], &[0x00, 0x2A]); // 42 in big-endian
    }

    #[test]
    fn player_speech_packet_has_correct_structure() {
        let packet = player_speech_packet(0x0000_0001, 0x0190, 0x00, 0x0035, 0x0003, "PlayerName", "Hi there!");
        assert_eq!(packet[0], 0x1C);
        let expected_len: u16 = 54;
        assert_eq!(&packet[1..3], &expected_len.to_be_bytes());
        assert_eq!(&packet[3..7], &[0x00, 0x00, 0x00, 0x01]);
        assert_eq!(&packet[7..9], &[0x01, 0x90]);
        assert_eq!(packet[9], 0x00);
        assert_eq!(&packet[10..12], &[0x00, 0x35]);
        assert_eq!(&packet[12..14], &[0x00, 0x03]);
        let name_field = &packet[14..44];
        assert_eq!(&name_field[..10], b"PlayerName");
        assert!(name_field[10..].iter().all(|&b| b == 0));
        assert_eq!(&packet[44..], b"Hi there!\0");
        assert_eq!(packet.len(), 54);
    }
}
