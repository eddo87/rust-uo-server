/// Speech types used in the Ultima Online protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SpeechType {
    Regular = 0x00,
    System = 0x01,
    Emote = 0x02,
    Label = 0x06,
    Focus = 0x07,
    Whisper = 0x08,
    Yell = 0x09,
    Spell = 0x0A,
    Guild = 0x0D,
    Alliance = 0x0E,
}

impl SpeechType {
    /// Parse a byte into a SpeechType. Returns None for unrecognized values.
    pub fn from_byte(byte: u8) -> Option<SpeechType> {
        match byte {
            0x00 => Some(SpeechType::Regular),
            0x01 => Some(SpeechType::System),
            0x02 => Some(SpeechType::Emote),
            0x06 => Some(SpeechType::Label),
            0x07 => Some(SpeechType::Focus),
            0x08 => Some(SpeechType::Whisper),
            0x09 => Some(SpeechType::Yell),
            0x0A => Some(SpeechType::Spell),
            0x0D => Some(SpeechType::Guild),
            0x0E => Some(SpeechType::Alliance),
            _ => None,
        }
    }
}

/// Represents a speech request parsed from client packet 0xAD.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeechRequest {
    pub speech_type: SpeechType,
    pub color: u16,
    pub font: u16,
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speech_type_from_byte_valid() {
        assert_eq!(SpeechType::from_byte(0x00), Some(SpeechType::Regular));
        assert_eq!(SpeechType::from_byte(0x01), Some(SpeechType::System));
        assert_eq!(SpeechType::from_byte(0x02), Some(SpeechType::Emote));
        assert_eq!(SpeechType::from_byte(0x06), Some(SpeechType::Label));
        assert_eq!(SpeechType::from_byte(0x07), Some(SpeechType::Focus));
        assert_eq!(SpeechType::from_byte(0x08), Some(SpeechType::Whisper));
        assert_eq!(SpeechType::from_byte(0x09), Some(SpeechType::Yell));
        assert_eq!(SpeechType::from_byte(0x0A), Some(SpeechType::Spell));
        assert_eq!(SpeechType::from_byte(0x0D), Some(SpeechType::Guild));
        assert_eq!(SpeechType::from_byte(0x0E), Some(SpeechType::Alliance));
    }

    #[test]
    fn speech_type_from_byte_invalid() {
        assert_eq!(SpeechType::from_byte(0x03), None);
        assert_eq!(SpeechType::from_byte(0xFF), None);
    }

    #[test]
    fn speech_type_as_u8() {
        assert_eq!(SpeechType::Regular as u8, 0x00);
        assert_eq!(SpeechType::System as u8, 0x01);
        assert_eq!(SpeechType::Emote as u8, 0x02);
        assert_eq!(SpeechType::Yell as u8, 0x09);
        assert_eq!(SpeechType::Guild as u8, 0x0D);
        assert_eq!(SpeechType::Alliance as u8, 0x0E);
    }

    #[test]
    fn speech_request_construction() {
        let req = SpeechRequest {
            speech_type: SpeechType::Regular,
            color: 0x0035,
            font: 0x0003,
            text: String::from("Hello world"),
        };
        assert_eq!(req.speech_type, SpeechType::Regular);
        assert_eq!(req.color, 0x0035);
        assert_eq!(req.font, 0x0003);
        assert_eq!(req.text, "Hello world");
    }
}
