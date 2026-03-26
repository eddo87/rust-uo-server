use byteorder::{BigEndian, ByteOrder};

/// Represents an item inside a container, used by container_contents_packet.
pub struct ContainerItem {
    pub serial: u32,
    pub item_id: u16,
    pub offset: u8,
    pub amount: u16,
    pub x: u16,
    pub y: u16,
    pub grid_index: u8,
    pub container_serial: u32,
    pub hue: u16,
}

/// Represents the result of parsing a pick-up item request (0x07).
#[derive(Debug, PartialEq)]
pub struct PickUpItemRequest {
    pub serial: u32,
    pub amount: u16,
}

/// Represents the result of parsing a drop item request (0x08).
#[derive(Debug, PartialEq)]
pub struct DropItemRequest {
    pub serial: u32,
    pub x: u16,
    pub y: u16,
    pub z: i8,
    pub container_serial: u32,
}

/// Represents the result of parsing an equip request (0x13).
#[derive(Debug, PartialEq)]
pub struct EquipRequest {
    pub item_serial: u32,
    pub layer: u8,
    pub mobile_serial: u32,
}

/// Open Container Gump packet (0x24).
/// Tells the client to open/display a container gump.
///
/// Format:
///   - 0x24        (1 byte)  packet id
///   - serial      (4 bytes) container serial
///   - gump_id     (2 bytes) gump model to display
///
/// Total: 7 bytes, fixed length.
pub fn open_container_gump_packet(serial: u32, gump_id: u16) -> Vec<u8> {
    let mut buf = vec![0u8; 7];
    buf[0] = 0x24;
    BigEndian::write_u32(&mut buf[1..5], serial);
    BigEndian::write_u16(&mut buf[5..7], gump_id);
    buf
}

/// Container Contents packet (0x3C).
/// Sends the full list of items inside a container to the client.
///
/// Format:
///   - 0x3C          (1 byte)  packet id
///   - length        (2 bytes) total packet length
///   - item_count    (2 bytes) number of items
///   For each item (20 bytes):
///     - serial            (4 bytes)
///     - item_id           (2 bytes)
///     - offset            (1 byte)
///     - amount            (2 bytes)
///     - x                 (2 bytes)
///     - y                 (2 bytes)
///     - grid_index        (1 byte)
///     - container_serial  (4 bytes)
///     - hue               (2 bytes)
///
/// Variable length: 5 + (item_count * 20) bytes.
pub fn container_contents_packet(items: &[ContainerItem]) -> Vec<u8> {
    let item_count = items.len() as u16;
    let packet_length: u16 = 5 + item_count * 20;
    let mut buf = vec![0u8; packet_length as usize];

    buf[0] = 0x3C;
    BigEndian::write_u16(&mut buf[1..3], packet_length);
    BigEndian::write_u16(&mut buf[3..5], item_count);

    for (i, item) in items.iter().enumerate() {
        let offset = 5 + i * 20;
        BigEndian::write_u32(&mut buf[offset..offset + 4], item.serial);
        BigEndian::write_u16(&mut buf[offset + 4..offset + 6], item.item_id);
        buf[offset + 6] = item.offset;
        BigEndian::write_u16(&mut buf[offset + 7..offset + 9], item.amount);
        BigEndian::write_u16(&mut buf[offset + 9..offset + 11], item.x);
        BigEndian::write_u16(&mut buf[offset + 11..offset + 13], item.y);
        buf[offset + 13] = item.grid_index;
        BigEndian::write_u32(&mut buf[offset + 14..offset + 18], item.container_serial);
        BigEndian::write_u16(&mut buf[offset + 18..offset + 20], item.hue);
    }

    buf
}

/// Add Item to Container packet (0x25).
/// Notifies the client that a single item has been added to a container.
///
/// Format:
///   - 0x25              (1 byte)  packet id
///   - serial            (4 bytes) item serial
///   - item_id           (2 bytes) item graphic/model id
///   - offset            (1 byte)
///   - amount            (2 bytes) stack amount
///   - x                 (2 bytes) x position in container
///   - y                 (2 bytes) y position in container
///   - grid_index        (1 byte)
///   - container_serial  (4 bytes) serial of the container
///   - hue               (2 bytes)
///
/// Total: 21 bytes, fixed length.
pub fn add_item_to_container_packet(
    serial: u32,
    item_id: u16,
    offset: u8,
    amount: u16,
    x: u16,
    y: u16,
    grid_index: u8,
    container_serial: u32,
    hue: u16,
) -> Vec<u8> {
    let mut buf = vec![0u8; 21];
    buf[0] = 0x25;
    BigEndian::write_u32(&mut buf[1..5], serial);
    BigEndian::write_u16(&mut buf[5..7], item_id);
    buf[7] = offset;
    BigEndian::write_u16(&mut buf[8..10], amount);
    BigEndian::write_u16(&mut buf[10..12], x);
    BigEndian::write_u16(&mut buf[12..14], y);
    buf[14] = grid_index;
    BigEndian::write_u32(&mut buf[15..19], container_serial);
    BigEndian::write_u16(&mut buf[19..21], hue);
    buf
}

/// Equip Item packet (0x2E).
/// Notifies the client that an item has been equipped on a mobile.
///
/// Format:
///   - 0x2E            (1 byte)  packet id
///   - item_serial     (4 bytes)
///   - item_id         (2 bytes) graphic/model id
///   - padding         (1 byte)  always 0x00
///   - layer           (1 byte)  equipment layer
///   - mobile_serial   (4 bytes) serial of the mobile wearing the item
///   - hue             (2 bytes)
///
/// Total: 15 bytes, fixed length.
pub fn equip_item_packet(
    item_serial: u32,
    item_id: u16,
    layer: u8,
    mobile_serial: u32,
    hue: u16,
) -> Vec<u8> {
    let mut buf = vec![0u8; 15];
    buf[0] = 0x2E;
    BigEndian::write_u32(&mut buf[1..5], item_serial);
    BigEndian::write_u16(&mut buf[5..7], item_id);
    buf[7] = 0x00; // padding
    buf[8] = layer;
    BigEndian::write_u32(&mut buf[9..13], mobile_serial);
    BigEndian::write_u16(&mut buf[13..15], hue);
    buf
}

/// Remove Item packet (0x1D).
/// Tells the client to remove an object from view.
///
/// Format:
///   - 0x1D    (1 byte)  packet id
///   - serial  (4 bytes)
///
/// Total: 5 bytes, fixed length.
pub fn remove_item_packet(serial: u32) -> Vec<u8> {
    let mut buf = vec![0u8; 5];
    buf[0] = 0x1D;
    BigEndian::write_u32(&mut buf[1..5], serial);
    buf
}

/// Parse Pick Up Item request (0x07).
/// The client sends this when the player picks up an item.
///
/// Expected input (after the 0x07 packet id byte has been consumed):
///   - serial  (4 bytes)
///   - amount  (2 bytes)
///
/// Total payload: 6 bytes.
pub fn parse_pick_up_item(data: &[u8]) -> PickUpItemRequest {
    let serial = BigEndian::read_u32(&data[0..4]);
    let amount = BigEndian::read_u16(&data[4..6]);
    PickUpItemRequest { serial, amount }
}

/// Parse Drop Item request (0x08).
/// The client sends this when the player drops an item.
///
/// Expected input (after the 0x08 packet id byte has been consumed):
///   - serial           (4 bytes)
///   - x                (2 bytes)
///   - y                (2 bytes)
///   - z                (1 byte, signed)
///   - container_serial (4 bytes)
///
/// Total payload: 13 bytes.
pub fn parse_drop_item(data: &[u8]) -> DropItemRequest {
    let serial = BigEndian::read_u32(&data[0..4]);
    let x = BigEndian::read_u16(&data[4..6]);
    let y = BigEndian::read_u16(&data[6..8]);
    let z = data[8] as i8;
    let container_serial = BigEndian::read_u32(&data[9..13]);
    DropItemRequest {
        serial,
        x,
        y,
        z,
        container_serial,
    }
}

/// Parse Equip Request (0x13).
/// The client sends this when the player equips an item.
///
/// Expected input (after the 0x13 packet id byte has been consumed):
///   - item_serial   (4 bytes)
///   - layer         (1 byte)
///   - mobile_serial (4 bytes)
///
/// Total payload: 9 bytes.
pub fn parse_equip_request(data: &[u8]) -> EquipRequest {
    let item_serial = BigEndian::read_u32(&data[0..4]);
    let layer = data[4];
    let mobile_serial = BigEndian::read_u32(&data[5..9]);
    EquipRequest {
        item_serial,
        layer,
        mobile_serial,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_container_gump_packet() {
        let packet = open_container_gump_packet(0x40001234, 0x003C);
        assert_eq!(packet.len(), 7);
        assert_eq!(packet[0], 0x24);
        // serial in big-endian
        assert_eq!(&packet[1..5], &[0x40, 0x00, 0x12, 0x34]);
        // gump_id in big-endian
        assert_eq!(&packet[5..7], &[0x00, 0x3C]);
    }

    #[test]
    fn test_container_contents_packet_empty() {
        let packet = container_contents_packet(&[]);
        assert_eq!(packet.len(), 5);
        assert_eq!(packet[0], 0x3C);
        // length = 5
        assert_eq!(&packet[1..3], &[0x00, 0x05]);
        // item count = 0
        assert_eq!(&packet[3..5], &[0x00, 0x00]);
    }

    #[test]
    fn test_container_contents_packet_with_items() {
        let items = vec![
            ContainerItem {
                serial: 0x40000001,
                item_id: 0x0F51,
                offset: 0x00,
                amount: 1,
                x: 50,
                y: 80,
                grid_index: 0,
                container_serial: 0x40001000,
                hue: 0x0000,
            },
            ContainerItem {
                serial: 0x40000002,
                item_id: 0x0E76,
                offset: 0x00,
                amount: 5,
                x: 100,
                y: 120,
                grid_index: 1,
                container_serial: 0x40001000,
                hue: 0x0035,
            },
        ];
        let packet = container_contents_packet(&items);
        // 5 header + 2*20 items = 45
        assert_eq!(packet.len(), 45);
        assert_eq!(packet[0], 0x3C);
        assert_eq!(BigEndian::read_u16(&packet[1..3]), 45);
        assert_eq!(BigEndian::read_u16(&packet[3..5]), 2);

        // First item
        assert_eq!(BigEndian::read_u32(&packet[5..9]), 0x40000001);
        assert_eq!(BigEndian::read_u16(&packet[9..11]), 0x0F51);
        assert_eq!(packet[11], 0x00); // offset
        assert_eq!(BigEndian::read_u16(&packet[12..14]), 1); // amount
        assert_eq!(BigEndian::read_u16(&packet[14..16]), 50); // x
        assert_eq!(BigEndian::read_u16(&packet[16..18]), 80); // y
        assert_eq!(packet[18], 0); // grid_index
        assert_eq!(BigEndian::read_u32(&packet[19..23]), 0x40001000);
        assert_eq!(BigEndian::read_u16(&packet[23..25]), 0x0000); // hue

        // Second item
        assert_eq!(BigEndian::read_u32(&packet[25..29]), 0x40000002);
        assert_eq!(BigEndian::read_u16(&packet[29..31]), 0x0E76);
        assert_eq!(BigEndian::read_u16(&packet[32..34]), 5); // amount
        assert_eq!(BigEndian::read_u16(&packet[34..36]), 100); // x
        assert_eq!(BigEndian::read_u16(&packet[36..38]), 120); // y
        assert_eq!(packet[38], 1); // grid_index
        assert_eq!(BigEndian::read_u16(&packet[43..45]), 0x0035); // hue
    }

    #[test]
    fn test_add_item_to_container_packet() {
        let packet = add_item_to_container_packet(
            0x40000ABC, // serial
            0x0F3F,     // item_id
            0x00,       // offset
            10,         // amount
            44,         // x
            65,         // y
            0x02,       // grid_index
            0x40001000, // container_serial
            0x0000,     // hue
        );
        assert_eq!(packet.len(), 21);
        assert_eq!(packet[0], 0x25);
        assert_eq!(BigEndian::read_u32(&packet[1..5]), 0x40000ABC);
        assert_eq!(BigEndian::read_u16(&packet[5..7]), 0x0F3F);
        assert_eq!(packet[7], 0x00);
        assert_eq!(BigEndian::read_u16(&packet[8..10]), 10);
        assert_eq!(BigEndian::read_u16(&packet[10..12]), 44);
        assert_eq!(BigEndian::read_u16(&packet[12..14]), 65);
        assert_eq!(packet[14], 0x02);
        assert_eq!(BigEndian::read_u32(&packet[15..19]), 0x40001000);
        assert_eq!(BigEndian::read_u16(&packet[19..21]), 0x0000);
    }

    #[test]
    fn test_equip_item_packet() {
        let packet = equip_item_packet(
            0x40000099, // item_serial
            0x13B9,     // item_id (Viking Sword)
            0x01,       // layer (one-handed weapon)
            0x00000001, // mobile_serial
            0x0000,     // hue
        );
        assert_eq!(packet.len(), 15);
        assert_eq!(packet[0], 0x2E);
        assert_eq!(BigEndian::read_u32(&packet[1..5]), 0x40000099);
        assert_eq!(BigEndian::read_u16(&packet[5..7]), 0x13B9);
        assert_eq!(packet[7], 0x00); // padding
        assert_eq!(packet[8], 0x01); // layer
        assert_eq!(BigEndian::read_u32(&packet[9..13]), 0x00000001);
        assert_eq!(BigEndian::read_u16(&packet[13..15]), 0x0000);
    }

    #[test]
    fn test_equip_item_packet_with_hue() {
        let packet = equip_item_packet(0x40000005, 0x1F03, 0x06, 0x00000002, 0x0455);
        assert_eq!(packet[8], 0x06); // layer
        assert_eq!(BigEndian::read_u16(&packet[13..15]), 0x0455); // hue
    }

    #[test]
    fn test_remove_item_packet() {
        let packet = remove_item_packet(0x40000FFF);
        assert_eq!(packet.len(), 5);
        assert_eq!(packet[0], 0x1D);
        assert_eq!(&packet[1..5], &[0x40, 0x00, 0x0F, 0xFF]);
    }

    #[test]
    fn test_parse_pick_up_item() {
        let data: [u8; 6] = [
            0x40, 0x00, 0x12, 0x34, // serial
            0x00, 0x0A, // amount = 10
        ];
        let result = parse_pick_up_item(&data);
        assert_eq!(
            result,
            PickUpItemRequest {
                serial: 0x40001234,
                amount: 10,
            }
        );
    }

    #[test]
    fn test_parse_drop_item() {
        let data: [u8; 13] = [
            0x40, 0x00, 0x12, 0x34, // serial
            0x00, 0x64, // x = 100
            0x00, 0xC8, // y = 200
            0xF6, // z = -10 (signed)
            0x40, 0x00, 0x10, 0x00, // container_serial
        ];
        let result = parse_drop_item(&data);
        assert_eq!(
            result,
            DropItemRequest {
                serial: 0x40001234,
                x: 100,
                y: 200,
                z: -10,
                container_serial: 0x40001000,
            }
        );
    }

    #[test]
    fn test_parse_drop_item_positive_z() {
        let data: [u8; 13] = [
            0x40, 0x00, 0x00, 0x01, // serial
            0x01, 0x00, // x = 256
            0x02, 0x00, // y = 512
            0x14, // z = 20 (positive)
            0x40, 0x00, 0x20, 0x00, // container_serial
        ];
        let result = parse_drop_item(&data);
        assert_eq!(result.z, 20);
        assert_eq!(result.container_serial, 0x40002000);
    }

    #[test]
    fn test_parse_equip_request() {
        let data: [u8; 9] = [
            0x40, 0x00, 0x00, 0x99, // item_serial
            0x01, // layer
            0x00, 0x00, 0x00, 0x01, // mobile_serial
        ];
        let result = parse_equip_request(&data);
        assert_eq!(
            result,
            EquipRequest {
                item_serial: 0x40000099,
                layer: 0x01,
                mobile_serial: 0x00000001,
            }
        );
    }

    #[test]
    fn test_open_container_gump_roundtrip_serial() {
        // Verify that serials survive the big-endian encoding roundtrip
        let serial = 0xDEADBEEF_u32;
        let packet = open_container_gump_packet(serial, 0x0000);
        let recovered = BigEndian::read_u32(&packet[1..5]);
        assert_eq!(recovered, serial);
    }

    #[test]
    fn test_remove_item_packet_min_serial() {
        let packet = remove_item_packet(0x00000000);
        assert_eq!(&packet[1..5], &[0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_remove_item_packet_max_serial() {
        let packet = remove_item_packet(0xFFFFFFFF);
        assert_eq!(&packet[1..5], &[0xFF, 0xFF, 0xFF, 0xFF]);
    }
}
