use byteorder::{BigEndian, ByteOrder};

// ---------------------------------------------------------------------------
// Effect types
// ---------------------------------------------------------------------------

/// Visual effect types used by the UO client (packet 0x70 / 0xC0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EffectType {
    /// A projectile that travels from source to target.
    Moving = 0x00,
    /// Lightning bolt rendered at a location.
    Lightning = 0x01,
    /// A stationary effect anchored to an absolute XYZ position.
    FixedXYZ = 0x02,
    /// A stationary effect anchored to a source serial (mobile/item).
    FixedFrom = 0x03,
}

// ---------------------------------------------------------------------------
// Sound‑effect constants
// ---------------------------------------------------------------------------

/// Well-known sound-effect IDs used by the classic Ultima Online client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundEffect(pub u16);

impl SoundEffect {
    pub const SPELL_FIZZLE: SoundEffect = SoundEffect(0x005C);
    pub const HEAL: SoundEffect = SoundEffect(0x01F2);
    pub const EXPLOSION: SoundEffect = SoundEffect(0x0207);
    pub const FIREBALL: SoundEffect = SoundEffect(0x015F);
    pub const LIGHTNING: SoundEffect = SoundEffect(0x0029);
    pub const MAGIC_ARROW: SoundEffect = SoundEffect(0x01E5);
    pub const POISON: SoundEffect = SoundEffect(0x0205);
    pub const CURE: SoundEffect = SoundEffect(0x01E0);
    pub const TELEPORT: SoundEffect = SoundEffect(0x01FE);
    pub const RECALL: SoundEffect = SoundEffect(0x01FC);
    pub const GATE: SoundEffect = SoundEffect(0x020E);
    pub const RESURRECT: SoundEffect = SoundEffect(0x0214);
    pub const SUMMON: SoundEffect = SoundEffect(0x0217);
    pub const BLESS: SoundEffect = SoundEffect(0x0202);
    pub const CURSE: SoundEffect = SoundEffect(0x01E1);
    pub const FLAMESTRIKE: SoundEffect = SoundEffect(0x0208);
}

// ---------------------------------------------------------------------------
// Packets
// ---------------------------------------------------------------------------

/// Builds a **Graphical Effect** packet (0x70, 28 bytes).
///
/// | Offset | Size | Field              |
/// |--------|------|--------------------|
/// |  0     |  1   | Packet ID (0x70)   |
/// |  1     |  1   | Effect type        |
/// |  2     |  4   | Source serial      |
/// |  6     |  4   | Target serial      |
/// | 10     |  2   | Item ID (graphic)  |
/// | 12     |  2   | Source X           |
/// | 14     |  2   | Source Y           |
/// | 16     |  1   | Source Z           |
/// | 17     |  2   | Target X           |
/// | 19     |  2   | Target Y           |
/// | 21     |  1   | Target Z           |
/// | 22     |  1   | Speed              |
/// | 23     |  1   | Duration           |
/// | 24     |  2   | (unknown/padding)  |
/// | 26     |  1   | Fixed direction    |
/// | 27     |  1   | Explode on impact  |
pub fn graphical_effect_packet(
    effect_type: EffectType,
    source_serial: u32,
    target_serial: u32,
    item_id: u16,
    source_x: u16,
    source_y: u16,
    source_z: i8,
    target_x: u16,
    target_y: u16,
    target_z: i8,
    speed: u8,
    duration: u8,
    fixed_direction: bool,
    explodes: bool,
) -> [u8; 28] {
    let mut buf: [u8; 28] = [0; 28];

    buf[0] = 0x70; // packet ID
    buf[1] = effect_type as u8;

    BigEndian::write_u32(&mut buf[2..6], source_serial);
    BigEndian::write_u32(&mut buf[6..10], target_serial);
    BigEndian::write_u16(&mut buf[10..12], item_id);
    BigEndian::write_u16(&mut buf[12..14], source_x);
    BigEndian::write_u16(&mut buf[14..16], source_y);
    buf[16] = source_z as u8;
    BigEndian::write_u16(&mut buf[17..19], target_x);
    BigEndian::write_u16(&mut buf[19..21], target_y);
    buf[21] = target_z as u8;
    buf[22] = speed;
    buf[23] = duration;
    // buf[24..26] reserved / padding – already zeroed
    buf[26] = if fixed_direction { 1 } else { 0 };
    buf[27] = if explodes { 1 } else { 0 };

    buf
}

/// Builds a **Play Sound** packet (0x54, 12 bytes).
///
/// | Offset | Size | Field                  |
/// |--------|------|------------------------|
/// |  0     |  1   | Packet ID (0x54)       |
/// |  1     |  1   | Mode (0 = normal)      |
/// |  2     |  2   | Sound model (effect)   |
/// |  4     |  2   | Volume                 |
/// |  6     |  2   | X                      |
/// |  8     |  2   | Y                      |
/// | 10     |  2   | Z (signed, as i16 BE)  |
pub fn play_sound_packet(
    sound: SoundEffect,
    x: u16,
    y: u16,
    z: i16,
) -> [u8; 12] {
    let mut buf: [u8; 12] = [0; 12];

    buf[0] = 0x54; // packet ID
    buf[1] = 0x00; // mode – normal
    BigEndian::write_u16(&mut buf[2..4], sound.0);
    BigEndian::write_u16(&mut buf[4..6], 0); // volume (unused by classic client)
    BigEndian::write_u16(&mut buf[6..8], x);
    BigEndian::write_u16(&mut buf[8..10], y);
    BigEndian::write_i16(&mut buf[10..12], z);

    buf
}

/// Builds a **Graphical Effect Extended** packet (0xC0, 36 bytes).
///
/// This is the extended version of 0x70 that adds hue, render‐mode, an
/// effect layer, and an optional explosion‐graphic field set.
///
/// Total length: 36 bytes (28‑byte base + 8 extension bytes).
///
/// Extension fields (offsets relative to start of packet):
///
/// | Offset | Size | Field                        |
/// |--------|------|------------------------------|
/// | 28     |  2   | Hue                          |
/// | 30     |  2   | Render mode                  |
/// | 32     |  2   | Effect (layer / sub-effect)  |
/// | 34     |  2   | Explode effect graphic       |
pub fn graphical_effect_extended_packet(
    effect_type: EffectType,
    source_serial: u32,
    target_serial: u32,
    item_id: u16,
    source_x: u16,
    source_y: u16,
    source_z: i8,
    target_x: u16,
    target_y: u16,
    target_z: i8,
    speed: u8,
    duration: u8,
    fixed_direction: bool,
    explodes: bool,
    hue: u16,
    render_mode: u16,
    effect: u16,
    explode_effect: u16,
) -> [u8; 36] {
    let mut buf: [u8; 36] = [0; 36];

    buf[0] = 0xC0; // packet ID
    buf[1] = effect_type as u8;

    BigEndian::write_u32(&mut buf[2..6], source_serial);
    BigEndian::write_u32(&mut buf[6..10], target_serial);
    BigEndian::write_u16(&mut buf[10..12], item_id);
    BigEndian::write_u16(&mut buf[12..14], source_x);
    BigEndian::write_u16(&mut buf[14..16], source_y);
    buf[16] = source_z as u8;
    BigEndian::write_u16(&mut buf[17..19], target_x);
    BigEndian::write_u16(&mut buf[19..21], target_y);
    buf[21] = target_z as u8;
    buf[22] = speed;
    buf[23] = duration;
    // buf[24..26] reserved
    buf[26] = if fixed_direction { 1 } else { 0 };
    buf[27] = if explodes { 1 } else { 0 };

    // Extension fields
    BigEndian::write_u16(&mut buf[28..30], hue);
    BigEndian::write_u16(&mut buf[30..32], render_mode);
    BigEndian::write_u16(&mut buf[32..34], effect);
    BigEndian::write_u16(&mut buf[34..36], explode_effect);

    buf
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convenience wrapper – play a sound at a world location.
pub fn play_sound_at(sound: SoundEffect, x: u16, y: u16, z: i16) -> [u8; 12] {
    play_sound_packet(sound, x, y, z)
}

/// Build a moving‑projectile effect from `source` to `target`.
pub fn send_moving_effect(
    source_serial: u32,
    target_serial: u32,
    item_id: u16,
    source_x: u16,
    source_y: u16,
    source_z: i8,
    target_x: u16,
    target_y: u16,
    target_z: i8,
    speed: u8,
    duration: u8,
    explodes: bool,
) -> [u8; 28] {
    graphical_effect_packet(
        EffectType::Moving,
        source_serial,
        target_serial,
        item_id,
        source_x,
        source_y,
        source_z,
        target_x,
        target_y,
        target_z,
        speed,
        duration,
        true,  // fixed direction
        explodes,
    )
}

/// Build a fixed (stationary) effect at an absolute XYZ position.
pub fn send_fixed_effect(
    item_id: u16,
    x: u16,
    y: u16,
    z: i8,
    speed: u8,
    duration: u8,
) -> [u8; 28] {
    graphical_effect_packet(
        EffectType::FixedXYZ,
        0,
        0,
        item_id,
        x,
        y,
        z,
        0,
        0,
        0,
        speed,
        duration,
        true,
        false,
    )
}

/// Build a lightning‑bolt effect on `target_serial`.
pub fn send_lightning_effect(target_serial: u32) -> [u8; 28] {
    graphical_effect_packet(
        EffectType::Lightning,
        0,
        target_serial,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        true,
        false,
    )
}

/// Build an explosion effect at a world location.
pub fn send_explosion_effect(
    item_id: u16,
    x: u16,
    y: u16,
    z: i8,
    speed: u8,
    duration: u8,
) -> [u8; 28] {
    graphical_effect_packet(
        EffectType::FixedXYZ,
        0,
        0,
        item_id,
        x,
        y,
        z,
        0,
        0,
        0,
        speed,
        duration,
        true,
        true, // explodes
    )
}

// ---------------------------------------------------------------------------
// Pre‑built spell effects
// ---------------------------------------------------------------------------

/// Flamestrike: fixed fire column (graphic 0x3709) + flamestrike sound.
pub fn flamestrike(x: u16, y: u16, z: i8) -> ([u8; 28], [u8; 12]) {
    let effect = send_fixed_effect(0x3709, x, y, z, 10, 30);
    let sound = play_sound_at(SoundEffect::FLAMESTRIKE, x, y, z as i16);
    (effect, sound)
}

/// Heal: fixed sparkle (graphic 0x376A) + heal sound.
pub fn heal(x: u16, y: u16, z: i8) -> ([u8; 28], [u8; 12]) {
    let effect = send_fixed_effect(0x376A, x, y, z, 9, 20);
    let sound = play_sound_at(SoundEffect::HEAL, x, y, z as i16);
    (effect, sound)
}

/// Magic Arrow: moving projectile (graphic 0x36E4) from source to target.
pub fn magic_arrow(
    source_serial: u32,
    target_serial: u32,
    sx: u16,
    sy: u16,
    sz: i8,
    tx: u16,
    ty: u16,
    tz: i8,
) -> ([u8; 28], [u8; 12]) {
    let effect = send_moving_effect(
        source_serial,
        target_serial,
        0x36E4,
        sx,
        sy,
        sz,
        tx,
        ty,
        tz,
        5,
        10,
        false,
    );
    let sound = play_sound_at(SoundEffect::MAGIC_ARROW, sx, sy, sz as i16);
    (effect, sound)
}

/// Poison: fixed green cloud (graphic 0x374A) + poison sound.
pub fn poison(x: u16, y: u16, z: i8) -> ([u8; 28], [u8; 12]) {
    let effect = send_fixed_effect(0x374A, x, y, z, 10, 15);
    let sound = play_sound_at(SoundEffect::POISON, x, y, z as i16);
    (effect, sound)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ----- EffectType enum -------------------------------------------------

    #[test]
    fn effect_type_discriminants() {
        assert_eq!(EffectType::Moving as u8, 0x00);
        assert_eq!(EffectType::Lightning as u8, 0x01);
        assert_eq!(EffectType::FixedXYZ as u8, 0x02);
        assert_eq!(EffectType::FixedFrom as u8, 0x03);
    }

    // ----- SoundEffect constants -------------------------------------------

    #[test]
    fn sound_effect_constants() {
        assert_eq!(SoundEffect::SPELL_FIZZLE.0, 0x005C);
        assert_eq!(SoundEffect::HEAL.0, 0x01F2);
        assert_eq!(SoundEffect::EXPLOSION.0, 0x0207);
        assert_eq!(SoundEffect::FIREBALL.0, 0x015F);
        assert_eq!(SoundEffect::LIGHTNING.0, 0x0029);
        assert_eq!(SoundEffect::MAGIC_ARROW.0, 0x01E5);
        assert_eq!(SoundEffect::POISON.0, 0x0205);
        assert_eq!(SoundEffect::CURE.0, 0x01E0);
        assert_eq!(SoundEffect::TELEPORT.0, 0x01FE);
        assert_eq!(SoundEffect::RECALL.0, 0x01FC);
        assert_eq!(SoundEffect::GATE.0, 0x020E);
        assert_eq!(SoundEffect::RESURRECT.0, 0x0214);
        assert_eq!(SoundEffect::SUMMON.0, 0x0217);
        assert_eq!(SoundEffect::BLESS.0, 0x0202);
        assert_eq!(SoundEffect::CURSE.0, 0x01E1);
        assert_eq!(SoundEffect::FLAMESTRIKE.0, 0x0208);
    }

    // ----- graphical_effect_packet (0x70) ----------------------------------

    #[test]
    fn graphical_effect_packet_id_and_length() {
        let pkt = graphical_effect_packet(
            EffectType::Moving, 0x00000001, 0x00000002,
            0x36E4, 100, 200, 10, 300, 400, 20,
            5, 15, true, false,
        );
        assert_eq!(pkt.len(), 28);
        assert_eq!(pkt[0], 0x70);
    }

    #[test]
    fn graphical_effect_packet_fields() {
        let pkt = graphical_effect_packet(
            EffectType::FixedXYZ,
            0xAABBCCDD,
            0x11223344,
            0x3709,
            1000, 2000, 30,
            3000, 4000, 40,
            8, 25,
            false, true,
        );

        // Effect type
        assert_eq!(pkt[1], EffectType::FixedXYZ as u8);
        // Source serial
        assert_eq!(BigEndian::read_u32(&pkt[2..6]), 0xAABBCCDD);
        // Target serial
        assert_eq!(BigEndian::read_u32(&pkt[6..10]), 0x11223344);
        // Item ID
        assert_eq!(BigEndian::read_u16(&pkt[10..12]), 0x3709);
        // Source coords
        assert_eq!(BigEndian::read_u16(&pkt[12..14]), 1000);
        assert_eq!(BigEndian::read_u16(&pkt[14..16]), 2000);
        assert_eq!(pkt[16] as i8, 30);
        // Target coords
        assert_eq!(BigEndian::read_u16(&pkt[17..19]), 3000);
        assert_eq!(BigEndian::read_u16(&pkt[19..21]), 4000);
        assert_eq!(pkt[21] as i8, 40);
        // Speed & duration
        assert_eq!(pkt[22], 8);
        assert_eq!(pkt[23], 25);
        // Flags
        assert_eq!(pkt[26], 0); // fixed_direction = false
        assert_eq!(pkt[27], 1); // explodes = true
    }

    // ----- play_sound_packet (0x54) ----------------------------------------

    #[test]
    fn play_sound_packet_id_and_length() {
        let pkt = play_sound_packet(SoundEffect::HEAL, 500, 600, 15);
        assert_eq!(pkt.len(), 12);
        assert_eq!(pkt[0], 0x54);
    }

    #[test]
    fn play_sound_packet_fields() {
        let pkt = play_sound_packet(SoundEffect::EXPLOSION, 1234, 5678, -10);

        assert_eq!(pkt[1], 0x00); // mode
        assert_eq!(BigEndian::read_u16(&pkt[2..4]), SoundEffect::EXPLOSION.0);
        assert_eq!(BigEndian::read_u16(&pkt[4..6]), 0); // volume
        assert_eq!(BigEndian::read_u16(&pkt[6..8]), 1234);
        assert_eq!(BigEndian::read_u16(&pkt[8..10]), 5678);
        assert_eq!(BigEndian::read_i16(&pkt[10..12]), -10);
    }

    // ----- graphical_effect_extended_packet (0xC0) -------------------------

    #[test]
    fn graphical_effect_extended_packet_id_and_length() {
        let pkt = graphical_effect_extended_packet(
            EffectType::Moving, 1, 2, 0x36E4,
            100, 200, 10, 300, 400, 20,
            5, 15, true, false,
            0x0044, 0x0000, 0x0001, 0x0000,
        );
        assert_eq!(pkt.len(), 36);
        assert_eq!(pkt[0], 0xC0);
    }

    #[test]
    fn graphical_effect_extended_packet_extension_fields() {
        let pkt = graphical_effect_extended_packet(
            EffectType::FixedFrom,
            0x00000001, 0x00000000,
            0x3709,
            500, 600, 15,
            0, 0, 0,
            10, 30,
            true, false,
            0x0835, 0x0004, 0x000A, 0x36CB,
        );

        // Base fields
        assert_eq!(pkt[1], EffectType::FixedFrom as u8);
        assert_eq!(BigEndian::read_u32(&pkt[2..6]), 1);
        assert_eq!(BigEndian::read_u16(&pkt[10..12]), 0x3709);

        // Extension fields
        assert_eq!(BigEndian::read_u16(&pkt[28..30]), 0x0835); // hue
        assert_eq!(BigEndian::read_u16(&pkt[30..32]), 0x0004); // render mode
        assert_eq!(BigEndian::read_u16(&pkt[32..34]), 0x000A); // effect
        assert_eq!(BigEndian::read_u16(&pkt[34..36]), 0x36CB); // explode effect
    }

    // ----- Helpers ---------------------------------------------------------

    #[test]
    fn play_sound_at_delegates_correctly() {
        let a = play_sound_at(SoundEffect::HEAL, 10, 20, 5);
        let b = play_sound_packet(SoundEffect::HEAL, 10, 20, 5);
        assert_eq!(a, b);
    }

    #[test]
    fn send_moving_effect_sets_type() {
        let pkt = send_moving_effect(1, 2, 0x36E4, 0, 0, 0, 100, 200, 10, 5, 10, true);
        assert_eq!(pkt[0], 0x70);
        assert_eq!(pkt[1], EffectType::Moving as u8);
        assert_eq!(pkt[27], 1); // explodes
    }

    #[test]
    fn send_fixed_effect_sets_type() {
        let pkt = send_fixed_effect(0x3709, 500, 600, 15, 10, 30);
        assert_eq!(pkt[1], EffectType::FixedXYZ as u8);
        assert_eq!(BigEndian::read_u16(&pkt[10..12]), 0x3709);
        assert_eq!(BigEndian::read_u16(&pkt[12..14]), 500);
        assert_eq!(BigEndian::read_u16(&pkt[14..16]), 600);
        assert_eq!(pkt[16] as i8, 15);
    }

    #[test]
    fn send_lightning_effect_sets_type() {
        let pkt = send_lightning_effect(0xDEADBEEF);
        assert_eq!(pkt[1], EffectType::Lightning as u8);
        assert_eq!(BigEndian::read_u32(&pkt[6..10]), 0xDEADBEEF);
    }

    #[test]
    fn send_explosion_effect_explodes() {
        let pkt = send_explosion_effect(0x36B0, 100, 200, 5, 8, 20);
        assert_eq!(pkt[1], EffectType::FixedXYZ as u8);
        assert_eq!(pkt[27], 1); // explodes flag
    }

    // ----- Pre‑built spell effects -----------------------------------------

    #[test]
    fn flamestrike_returns_correct_packets() {
        let (eff, snd) = flamestrike(400, 500, 10);
        assert_eq!(eff[0], 0x70);
        assert_eq!(eff[1], EffectType::FixedXYZ as u8);
        assert_eq!(BigEndian::read_u16(&eff[10..12]), 0x3709);
        assert_eq!(snd[0], 0x54);
        assert_eq!(BigEndian::read_u16(&snd[2..4]), SoundEffect::FLAMESTRIKE.0);
    }

    #[test]
    fn heal_returns_correct_packets() {
        let (eff, snd) = heal(100, 200, 5);
        assert_eq!(BigEndian::read_u16(&eff[10..12]), 0x376A);
        assert_eq!(BigEndian::read_u16(&snd[2..4]), SoundEffect::HEAL.0);
    }

    #[test]
    fn magic_arrow_returns_correct_packets() {
        let (eff, snd) = magic_arrow(1, 2, 100, 200, 10, 300, 400, 20);
        assert_eq!(eff[1], EffectType::Moving as u8);
        assert_eq!(BigEndian::read_u16(&eff[10..12]), 0x36E4);
        assert_eq!(BigEndian::read_u32(&eff[2..6]), 1);   // source serial
        assert_eq!(BigEndian::read_u32(&eff[6..10]), 2);   // target serial
        assert_eq!(BigEndian::read_u16(&snd[2..4]), SoundEffect::MAGIC_ARROW.0);
    }

    #[test]
    fn poison_returns_correct_packets() {
        let (eff, snd) = poison(50, 60, -5);
        assert_eq!(BigEndian::read_u16(&eff[10..12]), 0x374A);
        assert_eq!(BigEndian::read_u16(&snd[2..4]), SoundEffect::POISON.0);
        assert_eq!(BigEndian::read_u16(&snd[6..8]), 50);
        assert_eq!(BigEndian::read_u16(&snd[8..10]), 60);
        assert_eq!(BigEndian::read_i16(&snd[10..12]), -5);
    }

    // ----- Negative Z values -----------------------------------------------

    #[test]
    fn negative_z_handled_in_sound_packet() {
        let pkt = play_sound_packet(SoundEffect::HEAL, 0, 0, -20);
        assert_eq!(BigEndian::read_i16(&pkt[10..12]), -20);
    }

    #[test]
    fn negative_z_handled_in_effect_packet() {
        let pkt = graphical_effect_packet(
            EffectType::FixedXYZ, 0, 0, 0x3709,
            0, 0, -15, 0, 0, -30,
            10, 30, true, false,
        );
        assert_eq!(pkt[16] as i8, -15);
        assert_eq!(pkt[21] as i8, -30);
    }
}
