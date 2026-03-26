use std::collections::HashMap;

use byteorder::{BigEndian, ByteOrder};

/// The type of target the server is requesting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetType {
    /// Target a specific object (item or mobile).
    Object = 0x00,
    /// Target a location on the ground.
    Ground = 0x01,
}

impl TargetType {
    pub fn from_u8(value: u8) -> Option<TargetType> {
        match value {
            0x00 => Some(TargetType::Object),
            0x01 => Some(TargetType::Ground),
            _ => None,
        }
    }
}

/// Flags that describe the intent of the targeting action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetFlags {
    None = 0x00,
    Harmful = 0x01,
    Beneficial = 0x02,
}

impl TargetFlags {
    pub fn from_u8(value: u8) -> Option<TargetFlags> {
        match value {
            0x00 => Some(TargetFlags::None),
            0x01 => Some(TargetFlags::Harmful),
            0x02 => Some(TargetFlags::Beneficial),
            _ => None,
        }
    }
}

/// Reason the targeting cursor was cancelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetCancelType {
    /// The user pressed Escape or otherwise cancelled.
    UserCancelled,
    /// The server cancelled the targeting cursor.
    ServerCancelled,
    /// The target request timed out.
    Timeout,
}

/// A response from the client after a targeting cursor was presented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetResponse {
    pub target_type: TargetType,
    pub cursor_id: u32,
    pub cursor_type: u8,
    pub target_serial: u32,
    pub x: u16,
    pub y: u16,
    pub z: i16,
    pub graphic_id: u16,
}

/// A server-initiated targeting request to present a cursor to the client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetRequest {
    pub target_type: TargetType,
    pub cursor_id: u32,
    pub flags: TargetFlags,
}

/// Builds a Target Cursor packet (0x6C) to send from the server to the client.
///
/// Packet 0x6C (server -> client):
///   Byte  0:       0x6C (packet ID)
///   Byte  1:       target type (0 = object, 1 = ground)
///   Bytes 2-5:     cursor ID (u32)
///   Byte  6:       cursor/target flags
///   Bytes 7-18:    zeroed (reserved for client response data)
///
/// Total length: 19 bytes.
pub fn build_target_cursor_packet(request: &TargetRequest) -> [u8; 19] {
    let mut buffer = [0u8; 19];

    buffer[0] = 0x6C; // packet ID
    buffer[1] = request.target_type as u8;
    BigEndian::write_u32(&mut buffer[2..6], request.cursor_id);
    buffer[6] = request.flags as u8;
    // Bytes 7..18 are zero (reserved), already set by default.

    buffer
}

/// Parses a Target Cursor response packet (0x6C) received from the client.
///
/// Packet 0x6C (client -> server):
///   Byte  0:       0x6C (packet ID)
///   Byte  1:       target type (0 = object, 1 = ground)
///   Bytes 2-5:     cursor ID (u32)
///   Byte  6:       cursor type
///   Bytes 7-10:    target serial (u32)
///   Bytes 11-12:   x coordinate (u16)
///   Bytes 13-14:   y coordinate (u16)
///   Bytes 15-16:   z coordinate (i16, big-endian)
///   Bytes 17-18:   graphic/model ID (u16)
///
/// Total length: 19 bytes.
///
/// Returns `None` if the data is too short or contains an invalid target type.
pub fn parse_target_response(data: &[u8]) -> Option<TargetResponse> {
    if data.len() < 19 {
        return None;
    }

    // data[0] is the packet ID 0x6C (already consumed or verified by caller,
    // but we include it in the layout for completeness).
    let target_type = TargetType::from_u8(data[1])?;
    let cursor_id = BigEndian::read_u32(&data[2..6]);
    let cursor_type = data[6];
    let target_serial = BigEndian::read_u32(&data[7..11]);
    let x = BigEndian::read_u16(&data[11..13]);
    let y = BigEndian::read_u16(&data[13..15]);
    let z = BigEndian::read_i16(&data[15..17]);
    let graphic_id = BigEndian::read_u16(&data[17..19]);

    Some(TargetResponse {
        target_type,
        cursor_id,
        cursor_type,
        target_serial,
        x,
        y,
        z,
        graphic_id,
    })
}

/// Manages active targeting requests, keyed by a unique player/session ID.
pub struct TargetManager {
    active_targets: HashMap<u64, TargetRequest>,
}

impl TargetManager {
    pub fn new() -> TargetManager {
        TargetManager {
            active_targets: HashMap::new(),
        }
    }

    /// Registers a new targeting request for the given session.
    /// Returns the packet bytes to send to the client.
    /// If there is already an active target for this session, it is replaced.
    pub fn begin_target(&mut self, session_id: u64, request: TargetRequest) -> [u8; 19] {
        let packet = build_target_cursor_packet(&request);
        self.active_targets.insert(session_id, request);
        packet
    }

    /// Handles a targeting response from the client.
    /// Returns `Some((TargetRequest, TargetResponse))` if there was an active target
    /// for the session and the cursor IDs match. The active target is removed.
    /// Returns `None` if no matching active target was found.
    pub fn handle_response(
        &mut self,
        session_id: u64,
        response: TargetResponse,
    ) -> Option<(TargetRequest, TargetResponse)> {
        if let Some(request) = self.active_targets.get(&session_id) {
            if request.cursor_id == response.cursor_id {
                let request = self.active_targets.remove(&session_id).unwrap();
                return Some((request, response));
            }
        }
        None
    }

    /// Cancels any active targeting request for the given session.
    /// Returns the cancelled `TargetRequest` and the `TargetCancelType`, or `None`
    /// if there was no active target.
    pub fn cancel_target(
        &mut self,
        session_id: u64,
        cancel_type: TargetCancelType,
    ) -> Option<(TargetRequest, TargetCancelType)> {
        self.active_targets
            .remove(&session_id)
            .map(|req| (req, cancel_type))
    }

    /// Returns `true` if the given session has an active targeting cursor.
    pub fn has_active_target(&self, session_id: u64) -> bool {
        self.active_targets.contains_key(&session_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- TargetType tests ---

    #[test]
    fn target_type_from_u8_valid() {
        assert_eq!(TargetType::from_u8(0x00), Some(TargetType::Object));
        assert_eq!(TargetType::from_u8(0x01), Some(TargetType::Ground));
    }

    #[test]
    fn target_type_from_u8_invalid() {
        assert_eq!(TargetType::from_u8(0x02), None);
        assert_eq!(TargetType::from_u8(0xFF), None);
    }

    // --- TargetFlags tests ---

    #[test]
    fn target_flags_from_u8_valid() {
        assert_eq!(TargetFlags::from_u8(0x00), Some(TargetFlags::None));
        assert_eq!(TargetFlags::from_u8(0x01), Some(TargetFlags::Harmful));
        assert_eq!(TargetFlags::from_u8(0x02), Some(TargetFlags::Beneficial));
    }

    #[test]
    fn target_flags_from_u8_invalid() {
        assert_eq!(TargetFlags::from_u8(0x03), None);
        assert_eq!(TargetFlags::from_u8(0xFF), None);
    }

    // --- build_target_cursor_packet tests ---

    #[test]
    fn build_target_cursor_packet_object_harmful() {
        let request = TargetRequest {
            target_type: TargetType::Object,
            cursor_id: 0xDEADBEEF,
            flags: TargetFlags::Harmful,
        };

        let packet = build_target_cursor_packet(&request);

        assert_eq!(packet.len(), 19);
        assert_eq!(packet[0], 0x6C); // packet ID
        assert_eq!(packet[1], 0x00); // target type: Object
        assert_eq!(&packet[2..6], &[0xDE, 0xAD, 0xBE, 0xEF]); // cursor ID
        assert_eq!(packet[6], 0x01); // flags: Harmful
        // bytes 7..18 should all be zero
        assert_eq!(&packet[7..19], &[0u8; 12]);
    }

    #[test]
    fn build_target_cursor_packet_ground_beneficial() {
        let request = TargetRequest {
            target_type: TargetType::Ground,
            cursor_id: 0x00000001,
            flags: TargetFlags::Beneficial,
        };

        let packet = build_target_cursor_packet(&request);

        assert_eq!(packet[0], 0x6C);
        assert_eq!(packet[1], 0x01); // target type: Ground
        assert_eq!(BigEndian::read_u32(&packet[2..6]), 1);
        assert_eq!(packet[6], 0x02); // flags: Beneficial
    }

    #[test]
    fn build_target_cursor_packet_none_flags() {
        let request = TargetRequest {
            target_type: TargetType::Object,
            cursor_id: 0,
            flags: TargetFlags::None,
        };

        let packet = build_target_cursor_packet(&request);

        assert_eq!(packet[0], 0x6C);
        assert_eq!(packet[1], 0x00);
        assert_eq!(BigEndian::read_u32(&packet[2..6]), 0);
        assert_eq!(packet[6], 0x00);
    }

    // --- parse_target_response tests ---

    #[test]
    fn parse_target_response_valid_object() {
        let mut data = [0u8; 19];
        data[0] = 0x6C; // packet ID
        data[1] = 0x00; // target type: Object
        BigEndian::write_u32(&mut data[2..6], 0xDEADBEEF); // cursor ID
        data[6] = 0x01; // cursor type
        BigEndian::write_u32(&mut data[7..11], 0x00001234); // target serial
        BigEndian::write_u16(&mut data[11..13], 1000); // x
        BigEndian::write_u16(&mut data[13..15], 2000); // y
        BigEndian::write_i16(&mut data[15..17], -10); // z
        BigEndian::write_u16(&mut data[17..19], 0x0190); // graphic ID

        let response = parse_target_response(&data).unwrap();

        assert_eq!(response.target_type, TargetType::Object);
        assert_eq!(response.cursor_id, 0xDEADBEEF);
        assert_eq!(response.cursor_type, 0x01);
        assert_eq!(response.target_serial, 0x00001234);
        assert_eq!(response.x, 1000);
        assert_eq!(response.y, 2000);
        assert_eq!(response.z, -10);
        assert_eq!(response.graphic_id, 0x0190);
    }

    #[test]
    fn parse_target_response_valid_ground() {
        let mut data = [0u8; 19];
        data[0] = 0x6C;
        data[1] = 0x01; // target type: Ground
        BigEndian::write_u32(&mut data[2..6], 42);
        data[6] = 0x00;
        BigEndian::write_u32(&mut data[7..11], 0);
        BigEndian::write_u16(&mut data[11..13], 500);
        BigEndian::write_u16(&mut data[13..15], 600);
        BigEndian::write_i16(&mut data[15..17], 15);
        BigEndian::write_u16(&mut data[17..19], 0x0EED); // land tile graphic

        let response = parse_target_response(&data).unwrap();

        assert_eq!(response.target_type, TargetType::Ground);
        assert_eq!(response.cursor_id, 42);
        assert_eq!(response.x, 500);
        assert_eq!(response.y, 600);
        assert_eq!(response.z, 15);
        assert_eq!(response.graphic_id, 0x0EED);
    }

    #[test]
    fn parse_target_response_too_short() {
        let data = [0u8; 18]; // one byte too short
        assert!(parse_target_response(&data).is_none());
    }

    #[test]
    fn parse_target_response_empty() {
        let data: [u8; 0] = [];
        assert!(parse_target_response(&data).is_none());
    }

    #[test]
    fn parse_target_response_invalid_target_type() {
        let mut data = [0u8; 19];
        data[0] = 0x6C;
        data[1] = 0xFF; // invalid target type
        assert!(parse_target_response(&data).is_none());
    }

    // --- round-trip test ---

    #[test]
    fn build_then_parse_round_trip() {
        let request = TargetRequest {
            target_type: TargetType::Ground,
            cursor_id: 0x12345678,
            flags: TargetFlags::Beneficial,
        };

        let packet = build_target_cursor_packet(&request);

        // The server-to-client packet can be partially parsed as a response
        // (the reserved fields will be zero).
        let response = parse_target_response(&packet).unwrap();
        assert_eq!(response.target_type, request.target_type);
        assert_eq!(response.cursor_id, request.cursor_id);
        assert_eq!(response.target_serial, 0);
        assert_eq!(response.x, 0);
        assert_eq!(response.y, 0);
        assert_eq!(response.z, 0);
        assert_eq!(response.graphic_id, 0);
    }

    // --- TargetManager tests ---

    #[test]
    fn manager_begin_target_and_has_active() {
        let mut manager = TargetManager::new();
        let session_id = 1u64;

        assert!(!manager.has_active_target(session_id));

        let request = TargetRequest {
            target_type: TargetType::Object,
            cursor_id: 100,
            flags: TargetFlags::Harmful,
        };

        let packet = manager.begin_target(session_id, request);

        assert!(manager.has_active_target(session_id));
        assert_eq!(packet[0], 0x6C);
        assert_eq!(BigEndian::read_u32(&packet[2..6]), 100);
    }

    #[test]
    fn manager_handle_response_matching() {
        let mut manager = TargetManager::new();
        let session_id = 42u64;

        let request = TargetRequest {
            target_type: TargetType::Object,
            cursor_id: 0xABCD,
            flags: TargetFlags::None,
        };

        manager.begin_target(session_id, request);

        let response = TargetResponse {
            target_type: TargetType::Object,
            cursor_id: 0xABCD,
            cursor_type: 0,
            target_serial: 0x1111,
            x: 100,
            y: 200,
            z: 5,
            graphic_id: 0x0190,
        };

        let result = manager.handle_response(session_id, response);
        assert!(result.is_some());

        let (req, resp) = result.unwrap();
        assert_eq!(req.cursor_id, 0xABCD);
        assert_eq!(resp.target_serial, 0x1111);

        // After handling, the active target should be removed.
        assert!(!manager.has_active_target(session_id));
    }

    #[test]
    fn manager_handle_response_wrong_cursor_id() {
        let mut manager = TargetManager::new();
        let session_id = 1u64;

        let request = TargetRequest {
            target_type: TargetType::Object,
            cursor_id: 0xAAAA,
            flags: TargetFlags::Harmful,
        };

        manager.begin_target(session_id, request);

        let response = TargetResponse {
            target_type: TargetType::Object,
            cursor_id: 0xBBBB, // does not match
            cursor_type: 0,
            target_serial: 0x1111,
            x: 0,
            y: 0,
            z: 0,
            graphic_id: 0,
        };

        let result = manager.handle_response(session_id, response);
        assert!(result.is_none());

        // Target should still be active since it wasn't matched.
        assert!(manager.has_active_target(session_id));
    }

    #[test]
    fn manager_handle_response_no_active_target() {
        let mut manager = TargetManager::new();

        let response = TargetResponse {
            target_type: TargetType::Object,
            cursor_id: 1,
            cursor_type: 0,
            target_serial: 0x1111,
            x: 0,
            y: 0,
            z: 0,
            graphic_id: 0,
        };

        let result = manager.handle_response(999, response);
        assert!(result.is_none());
    }

    #[test]
    fn manager_cancel_target_active() {
        let mut manager = TargetManager::new();
        let session_id = 10u64;

        let request = TargetRequest {
            target_type: TargetType::Ground,
            cursor_id: 55,
            flags: TargetFlags::Beneficial,
        };

        manager.begin_target(session_id, request);
        assert!(manager.has_active_target(session_id));

        let result = manager.cancel_target(session_id, TargetCancelType::UserCancelled);
        assert!(result.is_some());

        let (req, cancel_type) = result.unwrap();
        assert_eq!(req.cursor_id, 55);
        assert_eq!(cancel_type, TargetCancelType::UserCancelled);

        assert!(!manager.has_active_target(session_id));
    }

    #[test]
    fn manager_cancel_target_server_cancelled() {
        let mut manager = TargetManager::new();
        let session_id = 20u64;

        let request = TargetRequest {
            target_type: TargetType::Object,
            cursor_id: 77,
            flags: TargetFlags::None,
        };

        manager.begin_target(session_id, request);

        let result = manager.cancel_target(session_id, TargetCancelType::ServerCancelled);
        assert!(result.is_some());

        let (_, cancel_type) = result.unwrap();
        assert_eq!(cancel_type, TargetCancelType::ServerCancelled);
    }

    #[test]
    fn manager_cancel_target_timeout() {
        let mut manager = TargetManager::new();
        let session_id = 30u64;

        let request = TargetRequest {
            target_type: TargetType::Object,
            cursor_id: 88,
            flags: TargetFlags::Harmful,
        };

        manager.begin_target(session_id, request);

        let result = manager.cancel_target(session_id, TargetCancelType::Timeout);
        assert!(result.is_some());

        let (_, cancel_type) = result.unwrap();
        assert_eq!(cancel_type, TargetCancelType::Timeout);
    }

    #[test]
    fn manager_cancel_target_no_active() {
        let mut manager = TargetManager::new();

        let result = manager.cancel_target(999, TargetCancelType::UserCancelled);
        assert!(result.is_none());
    }

    #[test]
    fn manager_begin_target_replaces_existing() {
        let mut manager = TargetManager::new();
        let session_id = 1u64;

        let first = TargetRequest {
            target_type: TargetType::Object,
            cursor_id: 100,
            flags: TargetFlags::Harmful,
        };

        let second = TargetRequest {
            target_type: TargetType::Ground,
            cursor_id: 200,
            flags: TargetFlags::Beneficial,
        };

        manager.begin_target(session_id, first);
        manager.begin_target(session_id, second);

        // The response should match the second request.
        let response = TargetResponse {
            target_type: TargetType::Ground,
            cursor_id: 200,
            cursor_type: 0,
            target_serial: 0,
            x: 50,
            y: 60,
            z: 0,
            graphic_id: 0,
        };

        let result = manager.handle_response(session_id, response);
        assert!(result.is_some());
        let (req, _) = result.unwrap();
        assert_eq!(req.cursor_id, 200);
        assert_eq!(req.target_type, TargetType::Ground);
    }

    #[test]
    fn manager_multiple_sessions() {
        let mut manager = TargetManager::new();

        let req1 = TargetRequest {
            target_type: TargetType::Object,
            cursor_id: 1,
            flags: TargetFlags::Harmful,
        };

        let req2 = TargetRequest {
            target_type: TargetType::Ground,
            cursor_id: 2,
            flags: TargetFlags::Beneficial,
        };

        manager.begin_target(100, req1);
        manager.begin_target(200, req2);

        assert!(manager.has_active_target(100));
        assert!(manager.has_active_target(200));
        assert!(!manager.has_active_target(300));

        // Cancel one, the other should remain.
        manager.cancel_target(100, TargetCancelType::ServerCancelled);
        assert!(!manager.has_active_target(100));
        assert!(manager.has_active_target(200));
    }
}
