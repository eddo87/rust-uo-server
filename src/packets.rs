use std::collections::HashMap;

/// Type alias for a UO packet identifier (first byte of each packet).
pub type PacketId = u8;

/// Direction a packet travels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketDirection {
    ClientToServer,
    ServerToClient,
    Bidirectional,
}

/// Metadata for a single UO packet type.
#[derive(Debug, Clone)]
pub struct PacketInfo {
    /// The packet id byte.
    pub id: PacketId,
    /// Human-readable name.
    pub name: &'static str,
    /// Which direction the packet flows.
    pub direction: PacketDirection,
    /// `Some(n)` for fixed-length packets (total length including the id byte),
    /// `None` for variable-length packets whose length is encoded in bytes 1-2.
    pub fixed_length: Option<usize>,
}

/// A registry that maps packet ids to their metadata.
pub struct PacketRegistry {
    packets: HashMap<PacketId, PacketInfo>,
}

impl PacketRegistry {
    /// Create a new registry pre-populated with known UO packets.
    pub fn new() -> Self {
        let entries: Vec<PacketInfo> = vec![
            PacketInfo {
                id: 0x02,
                name: "Movement Request",
                direction: PacketDirection::ClientToServer,
                fixed_length: Some(7),
            },
            PacketInfo {
                id: 0x06,
                name: "Double Click",
                direction: PacketDirection::ClientToServer,
                fixed_length: Some(5),
            },
            PacketInfo {
                id: 0x07,
                name: "Pick Up Item",
                direction: PacketDirection::ClientToServer,
                fixed_length: Some(7),
            },
            PacketInfo {
                id: 0x08,
                name: "Drop Item",
                direction: PacketDirection::ClientToServer,
                fixed_length: Some(14),
            },
            PacketInfo {
                id: 0x09,
                name: "Single Click",
                direction: PacketDirection::ClientToServer,
                fixed_length: Some(5),
            },
            PacketInfo {
                id: 0x12,
                name: "Skill/Macro Use",
                direction: PacketDirection::ClientToServer,
                fixed_length: None, // variable
            },
            PacketInfo {
                id: 0x22,
                name: "Movement Ack",
                direction: PacketDirection::ServerToClient,
                fixed_length: Some(3),
            },
            PacketInfo {
                id: 0x34,
                name: "Status Request",
                direction: PacketDirection::ClientToServer,
                fixed_length: Some(10),
            },
            PacketInfo {
                id: 0x3A,
                name: "Skill Lock Change",
                direction: PacketDirection::ClientToServer,
                fixed_length: Some(6),
            },
            PacketInfo {
                id: 0x5D,
                name: "Character Select",
                direction: PacketDirection::ClientToServer,
                fixed_length: Some(73),
            },
            PacketInfo {
                id: 0x73,
                name: "Ping",
                direction: PacketDirection::ClientToServer,
                fixed_length: Some(2),
            },
            PacketInfo {
                id: 0x80,
                name: "Account Login",
                direction: PacketDirection::ClientToServer,
                fixed_length: Some(62),
            },
            PacketInfo {
                id: 0x91,
                name: "Post-Login/Game Server Login",
                direction: PacketDirection::ClientToServer,
                fixed_length: Some(65),
            },
            PacketInfo {
                id: 0xA0,
                name: "Server Select",
                direction: PacketDirection::ClientToServer,
                fixed_length: Some(3),
            },
            PacketInfo {
                id: 0xAD,
                name: "Unicode Speech",
                direction: PacketDirection::ClientToServer,
                fixed_length: None, // variable
            },
            PacketInfo {
                id: 0xBF,
                name: "General Info",
                direction: PacketDirection::Bidirectional,
                fixed_length: None, // variable
            },
            PacketInfo {
                id: 0xEF,
                name: "Login Seed",
                direction: PacketDirection::ClientToServer,
                fixed_length: Some(21),
            },
        ];

        let mut packets = HashMap::new();
        for info in entries {
            packets.insert(info.id, info);
        }

        Self { packets }
    }

    /// Look up metadata for a packet id. Returns `None` for unknown packets.
    pub fn get_info(&self, id: PacketId) -> Option<&PacketInfo> {
        self.packets.get(&id)
    }

    /// Returns `true` if the packet id is registered.
    pub fn is_known(&self, id: PacketId) -> bool {
        self.packets.contains_key(&id)
    }

    /// Determine the total length of a packet given its raw bytes (starting
    /// from the packet id byte).
    ///
    /// - For fixed-length packets the length comes straight from the registry.
    /// - For variable-length packets the length is read as a big-endian u16
    ///   from bytes 1-2 of `data`.
    /// - Returns `None` when the packet id is unknown **or** when `data` is too
    ///   short to read the length field of a variable-length packet.
    pub fn get_packet_length(&self, id: PacketId, data: &[u8]) -> Option<usize> {
        let info = self.packets.get(&id)?;
        match info.fixed_length {
            Some(len) => Some(len),
            None => {
                // Variable-length: bytes 1-2 (after the id byte) hold the
                // total packet length as a big-endian u16.
                if data.len() < 3 {
                    return None;
                }
                let len = u16::from_be_bytes([data[1], data[2]]) as usize;
                Some(len)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Registry construction -------------------------------------------------

    #[test]
    fn new_registry_contains_all_known_packets() {
        let reg = PacketRegistry::new();
        let expected_ids: Vec<PacketId> = vec![
            0x02, 0x06, 0x07, 0x08, 0x09, 0x12, 0x22, 0x34, 0x3A, 0x5D, 0x73, 0x80, 0x91, 0xA0,
            0xAD, 0xBF, 0xEF,
        ];
        for id in &expected_ids {
            assert!(reg.is_known(*id), "packet 0x{:02X} should be known", id);
        }
        // total count must match
        assert_eq!(reg.packets.len(), expected_ids.len());
    }

    // ---- get_info --------------------------------------------------------------

    #[test]
    fn get_info_returns_correct_metadata_for_fixed_packet() {
        let reg = PacketRegistry::new();
        let info = reg.get_info(0x80).expect("0x80 should be registered");
        assert_eq!(info.id, 0x80);
        assert_eq!(info.name, "Account Login");
        assert_eq!(info.direction, PacketDirection::ClientToServer);
        assert_eq!(info.fixed_length, Some(62));
    }

    #[test]
    fn get_info_returns_correct_metadata_for_variable_packet() {
        let reg = PacketRegistry::new();
        let info = reg.get_info(0xAD).expect("0xAD should be registered");
        assert_eq!(info.id, 0xAD);
        assert_eq!(info.name, "Unicode Speech");
        assert_eq!(info.direction, PacketDirection::ClientToServer);
        assert_eq!(info.fixed_length, None);
    }

    #[test]
    fn get_info_returns_none_for_unknown_packet() {
        let reg = PacketRegistry::new();
        assert!(reg.get_info(0xFF).is_none());
    }

    #[test]
    fn get_info_bidirectional_packet() {
        let reg = PacketRegistry::new();
        let info = reg.get_info(0xBF).expect("0xBF should be registered");
        assert_eq!(info.direction, PacketDirection::Bidirectional);
        assert_eq!(info.fixed_length, None);
    }

    #[test]
    fn get_info_server_to_client_packet() {
        let reg = PacketRegistry::new();
        let info = reg.get_info(0x22).expect("0x22 should be registered");
        assert_eq!(info.direction, PacketDirection::ServerToClient);
        assert_eq!(info.fixed_length, Some(3));
    }

    // ---- is_known --------------------------------------------------------------

    #[test]
    fn is_known_returns_true_for_registered_packet() {
        let reg = PacketRegistry::new();
        assert!(reg.is_known(0xEF));
    }

    #[test]
    fn is_known_returns_false_for_unregistered_packet() {
        let reg = PacketRegistry::new();
        assert!(!reg.is_known(0x01));
        assert!(!reg.is_known(0xFF));
    }

    // ---- get_packet_length -----------------------------------------------------

    #[test]
    fn get_packet_length_fixed_ignores_data_contents() {
        let reg = PacketRegistry::new();
        // For a fixed-length packet the data contents are irrelevant.
        let data = [0x80, 0x00, 0x00]; // Account Login
        assert_eq!(reg.get_packet_length(0x80, &data), Some(62));
    }

    #[test]
    fn get_packet_length_variable_reads_from_bytes() {
        let reg = PacketRegistry::new();
        // Unicode Speech (0xAD) is variable. Encode a total length of 48.
        let data = [0xAD, 0x00, 0x30]; // 0x0030 = 48
        assert_eq!(reg.get_packet_length(0xAD, &data), Some(48));
    }

    #[test]
    fn get_packet_length_variable_large_value() {
        let reg = PacketRegistry::new();
        // General Info (0xBF) variable. Length = 0x0200 = 512
        let data = [0xBF, 0x02, 0x00];
        assert_eq!(reg.get_packet_length(0xBF, &data), Some(512));
    }

    #[test]
    fn get_packet_length_variable_too_short_data() {
        let reg = PacketRegistry::new();
        // Only 2 bytes provided, not enough to read the length field.
        let data = [0xAD, 0x00];
        assert_eq!(reg.get_packet_length(0xAD, &data), None);
    }

    #[test]
    fn get_packet_length_variable_single_byte() {
        let reg = PacketRegistry::new();
        let data = [0x12]; // Skill/Macro Use, variable, only id byte
        assert_eq!(reg.get_packet_length(0x12, &data), None);
    }

    #[test]
    fn get_packet_length_unknown_packet() {
        let reg = PacketRegistry::new();
        let data = [0xFF, 0x00, 0x10];
        assert_eq!(reg.get_packet_length(0xFF, &data), None);
    }

    // ---- Spot-check every registered fixed-length packet -----------------------

    #[test]
    fn fixed_lengths_match_specification() {
        let reg = PacketRegistry::new();
        let expected: Vec<(PacketId, usize)> = vec![
            (0x02, 7),
            (0x06, 5),
            (0x07, 7),
            (0x08, 14),
            (0x09, 5),
            (0x22, 3),
            (0x34, 10),
            (0x3A, 6),
            (0x5D, 73),
            (0x73, 2),
            (0x80, 62),
            (0x91, 65),
            (0xA0, 3),
            (0xEF, 21),
        ];
        for (id, len) in expected {
            let info = reg.get_info(id).unwrap_or_else(|| panic!("0x{:02X} missing", id));
            assert_eq!(
                info.fixed_length,
                Some(len),
                "packet 0x{:02X} should have fixed length {}",
                id,
                len
            );
        }
    }

    #[test]
    fn variable_length_packets_have_none() {
        let reg = PacketRegistry::new();
        for id in [0x12, 0xAD, 0xBF] {
            let info = reg.get_info(id).unwrap();
            assert_eq!(
                info.fixed_length, None,
                "packet 0x{:02X} should be variable-length",
                id
            );
        }
    }

    // ---- Names -----------------------------------------------------------------

    #[test]
    fn packet_names_are_populated() {
        let reg = PacketRegistry::new();
        for (_, info) in &reg.packets {
            assert!(
                !info.name.is_empty(),
                "packet 0x{:02X} should have a name",
                info.id
            );
        }
    }

    #[test]
    fn login_seed_has_correct_name() {
        let reg = PacketRegistry::new();
        let info = reg.get_info(0xEF).unwrap();
        assert_eq!(info.name, "Login Seed");
    }

    #[test]
    fn ping_packet_metadata() {
        let reg = PacketRegistry::new();
        let info = reg.get_info(0x73).unwrap();
        assert_eq!(info.name, "Ping");
        assert_eq!(info.fixed_length, Some(2));
        assert_eq!(info.direction, PacketDirection::ClientToServer);
    }
}
