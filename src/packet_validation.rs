use std::collections::{HashMap, HashSet, VecDeque};

/// Errors produced during packet validation.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    TooShort,
    TooLong,
    InvalidPacketId,
    InvalidLength,
    InvalidData(String),
    RateLimited,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::TooShort => write!(f, "Packet too short"),
            ValidationError::TooLong => write!(f, "Packet too long"),
            ValidationError::InvalidPacketId => write!(f, "Invalid packet ID"),
            ValidationError::InvalidLength => write!(f, "Invalid packet length"),
            ValidationError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            ValidationError::RateLimited => write!(f, "Rate limited"),
        }
    }
}

impl std::error::Error for ValidationError {}

// ---------------------------------------------------------------------------
// Known fixed-size packets in the UO protocol (packet id -> expected body
// length, i.e. the number of bytes *after* the 1-byte packet id).
// ---------------------------------------------------------------------------

/// Returns the expected body length (excluding the packet-id byte) for
/// fixed-size packets that the server currently handles.
fn known_packet_body_length(id: u8) -> Option<usize> {
    match id {
        0xEF => Some(20), // Encrypted Login Seed
        0x80 => Some(61), // Account Login Request
        0xA0 => Some(2),  // Server Select
        0x91 => Some(64), // Post Login
        0x73 => Some(1),  // Ping
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Map dimensions for coordinate validation.  Indices correspond to the map
// byte sent by the client.
// ---------------------------------------------------------------------------

/// (max_x, max_y) for each UO map.
const MAP_DIMENSIONS: [(u16, u16); 6] = [
    (7168, 4096), // Map 0 - Felucca
    (7168, 4096), // Map 1 - Trammel
    (2304, 1600), // Map 2 - Ilshenar
    (2560, 2048), // Map 3 - Malas
    (1448, 1448), // Map 4 - Tokuno Islands
    (1280, 4096), // Map 5 - Ter Mur
];

// ---------------------------------------------------------------------------
// PacketValidator
// ---------------------------------------------------------------------------

/// Configurable validator that checks size, packet-id allow-lists, and
/// delegates to the per-packet length checks.
pub struct PacketValidator {
    pub max_packet_size: usize,
    pub min_packet_size: usize,
    pub max_packets_per_second: u32,
    pub allowed_packet_ids: HashSet<u8>,
}

impl Default for PacketValidator {
    fn default() -> Self {
        Self {
            max_packet_size: 65535,
            min_packet_size: 1,
            max_packets_per_second: 100,
            allowed_packet_ids: HashSet::new(), // empty = allow all
        }
    }
}

impl PacketValidator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a validator pre-populated with the packet ids accepted during
    /// the login phase.
    pub fn login_phase() -> Self {
        let mut allowed = HashSet::new();
        allowed.insert(0xEF); // Encrypted Login Seed
        allowed.insert(0x80); // Account Login Request
        allowed.insert(0xA0); // Server Select
        allowed.insert(0x91); // Post Login
        allowed.insert(0x73); // Ping
        Self {
            allowed_packet_ids: allowed,
            ..Self::default()
        }
    }

    /// Validate an entire raw buffer (including the 1-byte packet id prefix).
    pub fn validate(&self, data: &[u8]) -> Result<(), ValidationError> {
        if data.len() < self.min_packet_size {
            return Err(ValidationError::TooShort);
        }
        if data.len() > self.max_packet_size {
            return Err(ValidationError::TooLong);
        }

        let packet_id = data[0];

        if !self.allowed_packet_ids.is_empty() && !self.allowed_packet_ids.contains(&packet_id) {
            return Err(ValidationError::InvalidPacketId);
        }

        // The body is everything after the packet-id byte.
        validate_packet_length(packet_id, &data[1..])?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// RateLimiter
// ---------------------------------------------------------------------------

/// Per-connection sliding-window rate limiter.
pub struct RateLimiter {
    /// Maximum number of packets allowed within `window_size_ms`.
    pub max_packets_per_second: u32,
    /// The sliding window duration in milliseconds.
    pub window_size_ms: u64,
    /// connection_id -> ordered list of packet timestamps (ms since epoch).
    windows: HashMap<u64, VecDeque<i64>>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self {
            max_packets_per_second: 100,
            window_size_ms: 1000,
            windows: HashMap::new(),
        }
    }
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_limits(max_packets_per_second: u32, window_size_ms: u64) -> Self {
        Self {
            max_packets_per_second,
            window_size_ms,
            windows: HashMap::new(),
        }
    }

    /// Record a packet arrival and check whether the connection has exceeded
    /// the rate limit.  Uses the provided `now_ms` timestamp so that the
    /// function is deterministic and testable.
    pub fn check_rate_with_time(
        &mut self,
        connection_id: u64,
        now_ms: i64,
    ) -> Result<(), ValidationError> {
        let window = self.windows.entry(connection_id).or_default();

        // Expire entries older than the sliding window.
        let cutoff = now_ms - self.window_size_ms as i64;
        while let Some(&front) = window.front() {
            if front <= cutoff {
                window.pop_front();
            } else {
                break;
            }
        }

        if window.len() as u32 >= self.max_packets_per_second {
            return Err(ValidationError::RateLimited);
        }

        window.push_back(now_ms);
        Ok(())
    }

    /// Convenience wrapper that uses the real wall-clock time.
    pub fn check_rate(&mut self, connection_id: u64) -> Result<(), ValidationError> {
        let now_ms = chrono::Utc::now().timestamp_millis();
        self.check_rate_with_time(connection_id, now_ms)
    }

    /// Remove tracking state for a disconnected connection.
    pub fn remove_connection(&mut self, connection_id: u64) {
        self.windows.remove(&connection_id);
    }
}

// ---------------------------------------------------------------------------
// Standalone validation helpers
// ---------------------------------------------------------------------------

/// Validate that `data` (the body *after* the packet-id byte) has the
/// correct length for the given `id`.  For unknown ids this is a no-op.
pub fn validate_packet_length(id: u8, data: &[u8]) -> Result<(), ValidationError> {
    if let Some(expected) = known_packet_body_length(id) {
        if data.len() < expected {
            return Err(ValidationError::TooShort);
        }
        // Allow trailing bytes (the buffer may contain subsequent packets).
    }
    Ok(())
}

/// Safely extract an ASCII string from a fixed-size field.
///
/// * Reads up to `max_len` bytes from `data`.
/// * Stops at the first null byte.
/// * Rejects non-ASCII bytes.
pub fn validate_string_field(data: &[u8], max_len: usize) -> Result<String, ValidationError> {
    if data.len() < max_len {
        return Err(ValidationError::TooShort);
    }

    let field = &data[..max_len];

    let mut result = String::new();
    for &b in field {
        if b == 0 {
            break;
        }
        if !b.is_ascii() {
            return Err(ValidationError::InvalidData(format!(
                "Non-ASCII byte 0x{:02X} in string field",
                b
            )));
        }
        // Reject control characters except common whitespace.
        if b.is_ascii_control() && b != b'\t' && b != b'\n' && b != b'\r' {
            return Err(ValidationError::InvalidData(format!(
                "Control character 0x{:02X} in string field",
                b
            )));
        }
        result.push(b as char);
    }

    Ok(result)
}

/// Validate that the given coordinates fall within the bounds for the
/// specified map.
pub fn validate_coordinates(x: u16, y: u16, map: u8) -> Result<(), ValidationError> {
    let map_idx = map as usize;
    if map_idx >= MAP_DIMENSIONS.len() {
        return Err(ValidationError::InvalidData(format!(
            "Unknown map index {}",
            map
        )));
    }

    let (max_x, max_y) = MAP_DIMENSIONS[map_idx];
    if x >= max_x || y >= max_y {
        return Err(ValidationError::InvalidData(format!(
            "Coordinates ({}, {}) out of bounds for map {} (max {}, {})",
            x, y, map, max_x, max_y
        )));
    }

    Ok(())
}

/// Maximum length of sanitised chat messages.
const MAX_CHAT_LENGTH: usize = 256;

/// Sanitize chat input by stripping control characters and limiting length.
pub fn sanitize_chat_input(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || *c == '\n')
        .take(MAX_CHAT_LENGTH)
        .collect()
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ----- ValidationError Display -----

    #[test]
    fn validation_error_display() {
        assert_eq!(ValidationError::TooShort.to_string(), "Packet too short");
        assert_eq!(
            ValidationError::InvalidData("bad".into()).to_string(),
            "Invalid data: bad"
        );
    }

    // ----- PacketValidator -----

    #[test]
    fn validator_rejects_empty_packet() {
        let v = PacketValidator::new();
        assert_eq!(v.validate(&[]), Err(ValidationError::TooShort));
    }

    #[test]
    fn validator_rejects_oversized_packet() {
        let mut v = PacketValidator::new();
        v.max_packet_size = 10;
        let data = vec![0xEF; 11];
        assert_eq!(v.validate(&data), Err(ValidationError::TooLong));
    }

    #[test]
    fn validator_rejects_disallowed_packet_id() {
        let v = PacketValidator::login_phase();
        // 0xFF is not in the login-phase allow list.
        let data = vec![0xFF, 0x00];
        assert_eq!(v.validate(&data), Err(ValidationError::InvalidPacketId));
    }

    #[test]
    fn validator_accepts_allowed_packet_with_correct_length() {
        let v = PacketValidator::login_phase();
        // 0xA0 (Server Select) has body length 2.
        let mut data = vec![0xA0];
        data.extend_from_slice(&[0x00; 2]);
        assert_eq!(v.validate(&data), Ok(()));
    }

    #[test]
    fn validator_rejects_packet_with_short_body() {
        let v = PacketValidator::login_phase();
        // 0xA0 expects 2 body bytes but we only supply 1.
        let data = vec![0xA0, 0x00];
        assert_eq!(v.validate(&data), Err(ValidationError::TooShort));
    }

    #[test]
    fn validator_allows_all_ids_when_allowlist_empty() {
        let v = PacketValidator::new();
        // Unknown id with some payload -- should pass (no length check for
        // unknown ids, and no allow-list restriction).
        let data = vec![0xFE, 0x01, 0x02];
        assert_eq!(v.validate(&data), Ok(()));
    }

    #[test]
    fn validator_accepts_encrypted_login_seed_packet() {
        let v = PacketValidator::login_phase();
        // 0xEF body is 20 bytes.
        let mut data = vec![0xEF];
        data.extend_from_slice(&[0x00; 20]);
        assert_eq!(v.validate(&data), Ok(()));
    }

    #[test]
    fn validator_accepts_account_login_request_packet() {
        let v = PacketValidator::login_phase();
        // 0x80 body is 61 bytes.
        let mut data = vec![0x80];
        data.extend_from_slice(&[0x00; 61]);
        assert_eq!(v.validate(&data), Ok(()));
    }

    #[test]
    fn validator_accepts_post_login_packet() {
        let v = PacketValidator::login_phase();
        // 0x91 body is 64 bytes.
        let mut data = vec![0x91];
        data.extend_from_slice(&[0x00; 64]);
        assert_eq!(v.validate(&data), Ok(()));
    }

    #[test]
    fn validator_rejects_short_encrypted_login_seed() {
        let v = PacketValidator::login_phase();
        let mut data = vec![0xEF];
        data.extend_from_slice(&[0x00; 10]); // only 10, need 20
        assert_eq!(v.validate(&data), Err(ValidationError::TooShort));
    }

    #[test]
    fn validator_custom_min_size() {
        let mut v = PacketValidator::new();
        v.min_packet_size = 5;
        let data = vec![0x01, 0x02, 0x03];
        assert_eq!(v.validate(&data), Err(ValidationError::TooShort));
    }

    // ----- validate_packet_length -----

    #[test]
    fn packet_length_ok_for_known_ids() {
        assert_eq!(validate_packet_length(0xEF, &[0; 20]), Ok(()));
        assert_eq!(validate_packet_length(0x80, &[0; 61]), Ok(()));
        assert_eq!(validate_packet_length(0xA0, &[0; 2]), Ok(()));
        assert_eq!(validate_packet_length(0x91, &[0; 64]), Ok(()));
        assert_eq!(validate_packet_length(0x73, &[0; 1]), Ok(()));
    }

    #[test]
    fn packet_length_too_short() {
        assert_eq!(
            validate_packet_length(0xEF, &[0; 5]),
            Err(ValidationError::TooShort)
        );
        assert_eq!(
            validate_packet_length(0x80, &[0; 10]),
            Err(ValidationError::TooShort)
        );
    }

    #[test]
    fn packet_length_unknown_id_always_ok() {
        assert_eq!(validate_packet_length(0xFF, &[]), Ok(()));
        assert_eq!(validate_packet_length(0x01, &[0; 100]), Ok(()));
    }

    #[test]
    fn packet_length_allows_trailing_bytes() {
        // Extra trailing bytes are fine (they could be the next packet).
        assert_eq!(validate_packet_length(0xA0, &[0; 100]), Ok(()));
    }

    // ----- RateLimiter -----

    #[test]
    fn rate_limiter_allows_under_limit() {
        let mut rl = RateLimiter::with_limits(5, 1000);
        for i in 0..5 {
            assert_eq!(rl.check_rate_with_time(1, 1000 + i), Ok(()));
        }
    }

    #[test]
    fn rate_limiter_blocks_over_limit() {
        let mut rl = RateLimiter::with_limits(3, 1000);
        assert_eq!(rl.check_rate_with_time(1, 1000), Ok(()));
        assert_eq!(rl.check_rate_with_time(1, 1001), Ok(()));
        assert_eq!(rl.check_rate_with_time(1, 1002), Ok(()));
        assert_eq!(
            rl.check_rate_with_time(1, 1003),
            Err(ValidationError::RateLimited)
        );
    }

    #[test]
    fn rate_limiter_sliding_window_expires_old_entries() {
        let mut rl = RateLimiter::with_limits(3, 1000);
        // Fill the window at t=0..2.
        assert_eq!(rl.check_rate_with_time(1, 0), Ok(()));
        assert_eq!(rl.check_rate_with_time(1, 1), Ok(()));
        assert_eq!(rl.check_rate_with_time(1, 2), Ok(()));
        // Blocked at t=500 (within window).
        assert_eq!(
            rl.check_rate_with_time(1, 500),
            Err(ValidationError::RateLimited)
        );
        // After the window has slid past t=0..2, we can send again.
        assert_eq!(rl.check_rate_with_time(1, 1001), Ok(()));
        assert_eq!(rl.check_rate_with_time(1, 1002), Ok(()));
        assert_eq!(rl.check_rate_with_time(1, 1003), Ok(()));
        // Blocked again.
        assert_eq!(
            rl.check_rate_with_time(1, 1004),
            Err(ValidationError::RateLimited)
        );
    }

    #[test]
    fn rate_limiter_independent_connections() {
        let mut rl = RateLimiter::with_limits(2, 1000);
        assert_eq!(rl.check_rate_with_time(1, 0), Ok(()));
        assert_eq!(rl.check_rate_with_time(1, 1), Ok(()));
        assert_eq!(
            rl.check_rate_with_time(1, 2),
            Err(ValidationError::RateLimited)
        );
        // Different connection is unaffected.
        assert_eq!(rl.check_rate_with_time(2, 2), Ok(()));
        assert_eq!(rl.check_rate_with_time(2, 3), Ok(()));
    }

    #[test]
    fn rate_limiter_remove_connection() {
        let mut rl = RateLimiter::with_limits(2, 1000);
        assert_eq!(rl.check_rate_with_time(1, 0), Ok(()));
        assert_eq!(rl.check_rate_with_time(1, 1), Ok(()));
        assert_eq!(
            rl.check_rate_with_time(1, 2),
            Err(ValidationError::RateLimited)
        );
        rl.remove_connection(1);
        // After removal, the connection starts fresh.
        assert_eq!(rl.check_rate_with_time(1, 3), Ok(()));
    }

    #[test]
    fn rate_limiter_default_limits() {
        let rl = RateLimiter::new();
        assert_eq!(rl.max_packets_per_second, 100);
        assert_eq!(rl.window_size_ms, 1000);
    }

    // ----- validate_string_field -----

    #[test]
    fn string_field_simple_ascii() {
        let data = b"Hello\0\0\0\0\0";
        assert_eq!(validate_string_field(data, 10), Ok("Hello".into()));
    }

    #[test]
    fn string_field_full_length_no_null() {
        let data = b"ABCDE";
        assert_eq!(validate_string_field(data, 5), Ok("ABCDE".into()));
    }

    #[test]
    fn string_field_empty_due_to_leading_null() {
        let data = b"\0ABCDE";
        assert_eq!(validate_string_field(data, 6), Ok("".into()));
    }

    #[test]
    fn string_field_rejects_non_ascii() {
        let data = &[0x80, 0x00, 0x00];
        assert!(matches!(
            validate_string_field(data, 3),
            Err(ValidationError::InvalidData(_))
        ));
    }

    #[test]
    fn string_field_rejects_control_chars() {
        // 0x01 is a control character that is not tab/newline/cr.
        let data = &[0x41, 0x01, 0x00];
        assert!(matches!(
            validate_string_field(data, 3),
            Err(ValidationError::InvalidData(_))
        ));
    }

    #[test]
    fn string_field_allows_tab() {
        let data = b"A\tB\0";
        assert_eq!(validate_string_field(data, 4), Ok("A\tB".into()));
    }

    #[test]
    fn string_field_too_short_buffer() {
        let data = b"AB";
        assert_eq!(validate_string_field(data, 5), Err(ValidationError::TooShort));
    }

    #[test]
    fn string_field_all_nulls() {
        let data = &[0x00; 10];
        assert_eq!(validate_string_field(data, 10), Ok("".into()));
    }

    // ----- validate_coordinates -----

    #[test]
    fn coordinates_valid_felucca() {
        assert_eq!(validate_coordinates(1000, 2000, 0), Ok(()));
    }

    #[test]
    fn coordinates_valid_trammel() {
        assert_eq!(validate_coordinates(7167, 4095, 1), Ok(()));
    }

    #[test]
    fn coordinates_valid_origin() {
        assert_eq!(validate_coordinates(0, 0, 0), Ok(()));
    }

    #[test]
    fn coordinates_out_of_bounds_x() {
        assert!(matches!(
            validate_coordinates(7168, 0, 0),
            Err(ValidationError::InvalidData(_))
        ));
    }

    #[test]
    fn coordinates_out_of_bounds_y() {
        assert!(matches!(
            validate_coordinates(0, 4096, 0),
            Err(ValidationError::InvalidData(_))
        ));
    }

    #[test]
    fn coordinates_both_out_of_bounds() {
        assert!(matches!(
            validate_coordinates(9999, 9999, 0),
            Err(ValidationError::InvalidData(_))
        ));
    }

    #[test]
    fn coordinates_invalid_map() {
        assert!(matches!(
            validate_coordinates(0, 0, 6),
            Err(ValidationError::InvalidData(_))
        ));
        assert!(matches!(
            validate_coordinates(0, 0, 255),
            Err(ValidationError::InvalidData(_))
        ));
    }

    #[test]
    fn coordinates_ilshenar_bounds() {
        // Map 2 max is (2304, 1600).
        assert_eq!(validate_coordinates(2303, 1599, 2), Ok(()));
        assert!(matches!(
            validate_coordinates(2304, 1599, 2),
            Err(ValidationError::InvalidData(_))
        ));
        assert!(matches!(
            validate_coordinates(2303, 1600, 2),
            Err(ValidationError::InvalidData(_))
        ));
    }

    #[test]
    fn coordinates_tokuno_bounds() {
        // Map 4 max is (1448, 1448).
        assert_eq!(validate_coordinates(1447, 1447, 4), Ok(()));
        assert!(matches!(
            validate_coordinates(1448, 0, 4),
            Err(ValidationError::InvalidData(_))
        ));
    }

    #[test]
    fn coordinates_ter_mur_bounds() {
        // Map 5 max is (1280, 4096).
        assert_eq!(validate_coordinates(1279, 4095, 5), Ok(()));
        assert!(matches!(
            validate_coordinates(1280, 0, 5),
            Err(ValidationError::InvalidData(_))
        ));
    }

    // ----- sanitize_chat_input -----

    #[test]
    fn sanitize_preserves_normal_text() {
        assert_eq!(sanitize_chat_input("Hello world!"), "Hello world!");
    }

    #[test]
    fn sanitize_strips_control_chars() {
        assert_eq!(sanitize_chat_input("AB\x01\x02CD"), "ABCD");
    }

    #[test]
    fn sanitize_preserves_newlines() {
        assert_eq!(sanitize_chat_input("line1\nline2"), "line1\nline2");
    }

    #[test]
    fn sanitize_strips_carriage_return() {
        assert_eq!(sanitize_chat_input("line1\r\nline2"), "line1\nline2");
    }

    #[test]
    fn sanitize_strips_null() {
        assert_eq!(sanitize_chat_input("AB\0CD"), "ABCD");
    }

    #[test]
    fn sanitize_limits_length() {
        let long = "A".repeat(500);
        let result = sanitize_chat_input(&long);
        assert_eq!(result.len(), MAX_CHAT_LENGTH);
    }

    #[test]
    fn sanitize_empty_input() {
        assert_eq!(sanitize_chat_input(""), "");
    }

    #[test]
    fn sanitize_only_control_chars() {
        assert_eq!(sanitize_chat_input("\x00\x01\x02\x03\x04"), "");
    }

    #[test]
    fn sanitize_unicode_preserved() {
        // Non-ASCII printable characters should survive.
        assert_eq!(sanitize_chat_input("cafe\u{0301}"), "cafe\u{0301}");
    }

    #[test]
    fn sanitize_tab_stripped() {
        // Tab (0x09) is a control character, so it should be stripped.
        assert_eq!(sanitize_chat_input("A\tB"), "AB");
    }

    #[test]
    fn sanitize_mixed_control_and_printable() {
        let input = "\x07Hello\x08 \x1Bworld\x7F!";
        assert_eq!(sanitize_chat_input(input), "Hello world!");
    }

    #[test]
    fn sanitize_length_limit_respects_char_boundaries() {
        // Each emoji is one char but multiple bytes.
        let input: String = std::iter::repeat('\u{1F600}').take(300).collect();
        let result = sanitize_chat_input(&input);
        assert_eq!(result.chars().count(), MAX_CHAT_LENGTH);
    }
}
