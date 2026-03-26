/// Represents the eight cardinal/ordinal directions in the UO protocol.
/// The wire values match the UO client protocol: 0x00 = North through 0x07 = Northwest.
/// Bit 0x80 is the "running" flag and is handled separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Direction {
    North = 0x00,
    Northeast = 0x01,
    East = 0x02,
    Southeast = 0x03,
    South = 0x04,
    Southwest = 0x05,
    West = 0x06,
    Northwest = 0x07,
}

impl Direction {
    /// Decode a raw direction byte from the wire.
    /// The lower 3 bits encode the direction; bit 0x80 indicates running.
    /// Returns `None` if the direction bits are invalid (should never happen
    /// with a well-behaved client since all 8 values are defined).
    pub fn from_byte(byte: u8) -> Option<Direction> {
        match byte & 0x07 {
            0x00 => Some(Direction::North),
            0x01 => Some(Direction::Northeast),
            0x02 => Some(Direction::East),
            0x03 => Some(Direction::Southeast),
            0x04 => Some(Direction::South),
            0x05 => Some(Direction::Southwest),
            0x06 => Some(Direction::West),
            0x07 => Some(Direction::Northwest),
            _ => None,
        }
    }

    /// Encode the direction back to its wire byte representation.
    pub fn to_byte(self) -> u8 {
        self as u8
    }
}

/// Whether the character is running (bit 0x80 set on the direction byte).
pub fn is_running(direction_byte: u8) -> bool {
    direction_byte & 0x80 != 0
}

/// Encode direction + running flag back into a single wire byte.
pub fn encode_direction(direction: Direction, running: bool) -> u8 {
    let mut byte = direction.to_byte();
    if running {
        byte |= 0x80;
    }
    byte
}

/// A position in the UO world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: u16,
    pub y: u16,
    pub z: i8,
}

/// A parsed 0x02 Movement Request from the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MovementRequest {
    pub direction: Direction,
    pub running: bool,
    pub sequence_number: u8,
    pub fastwalk_key: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_from_byte_all_directions() {
        assert_eq!(Direction::from_byte(0x00), Some(Direction::North));
        assert_eq!(Direction::from_byte(0x01), Some(Direction::Northeast));
        assert_eq!(Direction::from_byte(0x02), Some(Direction::East));
        assert_eq!(Direction::from_byte(0x03), Some(Direction::Southeast));
        assert_eq!(Direction::from_byte(0x04), Some(Direction::South));
        assert_eq!(Direction::from_byte(0x05), Some(Direction::Southwest));
        assert_eq!(Direction::from_byte(0x06), Some(Direction::West));
        assert_eq!(Direction::from_byte(0x07), Some(Direction::Northwest));
    }

    #[test]
    fn direction_from_byte_with_running_flag() {
        // Running flag (0x80) should be masked off when determining direction
        assert_eq!(Direction::from_byte(0x80), Some(Direction::North));
        assert_eq!(Direction::from_byte(0x83), Some(Direction::Southeast));
        assert_eq!(Direction::from_byte(0x87), Some(Direction::Northwest));
    }

    #[test]
    fn direction_to_byte_roundtrip() {
        for raw in 0x00..=0x07u8 {
            let dir = Direction::from_byte(raw).unwrap();
            assert_eq!(dir.to_byte(), raw);
        }
    }

    #[test]
    fn is_running_flag() {
        assert!(!is_running(0x00));
        assert!(!is_running(0x03));
        assert!(is_running(0x80));
        assert!(is_running(0x84));
    }

    #[test]
    fn encode_direction_without_running() {
        assert_eq!(encode_direction(Direction::North, false), 0x00);
        assert_eq!(encode_direction(Direction::South, false), 0x04);
    }

    #[test]
    fn encode_direction_with_running() {
        assert_eq!(encode_direction(Direction::North, true), 0x80);
        assert_eq!(encode_direction(Direction::East, true), 0x82);
        assert_eq!(encode_direction(Direction::Northwest, true), 0x87);
    }

    #[test]
    fn position_struct() {
        let pos = Position { x: 1602, y: 1591, z: 20 };
        assert_eq!(pos.x, 1602);
        assert_eq!(pos.y, 1591);
        assert_eq!(pos.z, 20);
    }

    #[test]
    fn movement_request_struct() {
        let req = MovementRequest {
            direction: Direction::South,
            running: true,
            sequence_number: 42,
            fastwalk_key: 0xDEADBEEF,
        };
        assert_eq!(req.direction, Direction::South);
        assert!(req.running);
        assert_eq!(req.sequence_number, 42);
        assert_eq!(req.fastwalk_key, 0xDEADBEEF);
    }
}
