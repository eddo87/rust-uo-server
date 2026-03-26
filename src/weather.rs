use std::time::Instant;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// The four seasons of Britannia.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Season {
    Spring,
    Summer,
    Fall,
    Winter,
}

/// Weather conditions that can be active in a region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherType {
    Clear,
    Cloudy,
    Rain,
    Storm,
    Snow,
    Fog,
}

/// Eight phases of the moon (used for both Trammel and Felucca).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoonPhase {
    NewMoon,
    WaxingCrescent,
    FirstQuarter,
    WaxingGibbous,
    FullMoon,
    WaningGibbous,
    ThirdQuarter,
    WaningCrescent,
}

impl MoonPhase {
    /// Return the phase for a given index (0..7), wrapping around.
    pub fn from_index(index: u8) -> MoonPhase {
        match index % 8 {
            0 => MoonPhase::NewMoon,
            1 => MoonPhase::WaxingCrescent,
            2 => MoonPhase::FirstQuarter,
            3 => MoonPhase::WaxingGibbous,
            4 => MoonPhase::FullMoon,
            5 => MoonPhase::WaningGibbous,
            6 => MoonPhase::ThirdQuarter,
            _ => MoonPhase::WaningCrescent,
        }
    }

    /// Convert to a 0-based index.
    pub fn to_index(self) -> u8 {
        match self {
            MoonPhase::NewMoon => 0,
            MoonPhase::WaxingCrescent => 1,
            MoonPhase::FirstQuarter => 2,
            MoonPhase::WaxingGibbous => 3,
            MoonPhase::FullMoon => 4,
            MoonPhase::WaningGibbous => 5,
            MoonPhase::ThirdQuarter => 6,
            MoonPhase::WaningCrescent => 7,
        }
    }
}

// ---------------------------------------------------------------------------
// TimeOfDay
// ---------------------------------------------------------------------------

/// Represents a time of day within the UO world (24-hour clock).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeOfDay {
    pub hour: u8,   // 0..23
    pub minute: u8, // 0..59
}

impl TimeOfDay {
    pub fn new(hour: u8, minute: u8) -> TimeOfDay {
        TimeOfDay {
            hour: hour % 24,
            minute: minute % 60,
        }
    }

    /// Total minutes since midnight.
    pub fn total_minutes(&self) -> u16 {
        self.hour as u16 * 60 + self.minute as u16
    }

    /// Daytime is 07:00 .. 20:59 (after dawn, before dusk ends).
    pub fn is_day(&self) -> bool {
        self.hour >= 7 && self.hour < 21
    }

    /// Night-time is 22:00 .. 04:59.
    pub fn is_night(&self) -> bool {
        self.hour >= 22 || self.hour < 5
    }

    /// Dawn is 05:00 .. 06:59.
    pub fn is_dawn(&self) -> bool {
        self.hour >= 5 && self.hour < 7
    }

    /// Dusk is 21:00 .. 21:59.
    pub fn is_dusk(&self) -> bool {
        self.hour >= 21 && self.hour < 22
    }
}

// ---------------------------------------------------------------------------
// UOClock  (1 real minute = 8 UO minutes)
// ---------------------------------------------------------------------------

/// The master game clock.
///
/// The ratio is 1 real-world minute = 8 UO minutes, so a full UO day
/// (24 * 60 = 1440 UO-minutes) passes in 180 real-world minutes (3 hours).
///
/// We also track a day counter which drives seasons and moon phases.
pub struct UOClock {
    /// The real-world instant when the clock was created / last reset.
    epoch: Instant,
    /// The UO-minute offset at epoch (lets us seed the starting time).
    start_uo_minutes: u64,
}

/// One UO day = 1440 UO-minutes.
const UO_MINUTES_PER_DAY: u64 = 24 * 60;

/// UO days per season (roughly 12 UO-days = 1 season, 48 UO-days = 1 year).
const UO_DAYS_PER_SEASON: u64 = 12;

/// Trammel moon cycle length in UO days.
const TRAMMEL_CYCLE_DAYS: u64 = 8;

/// Felucca moon cycle length in UO days (slightly longer).
const FELUCCA_CYCLE_DAYS: u64 = 10;

impl UOClock {
    /// Create a new clock starting at midnight of UO day 0.
    pub fn new() -> UOClock {
        UOClock {
            epoch: Instant::now(),
            start_uo_minutes: 0,
        }
    }

    /// Create a clock with a specific starting UO time (hour:minute on day 0).
    pub fn with_start_time(hour: u8, minute: u8) -> UOClock {
        UOClock {
            epoch: Instant::now(),
            start_uo_minutes: (hour % 24) as u64 * 60 + (minute % 60) as u64,
        }
    }

    /// Create a clock seeded at an arbitrary UO-minute count (useful for tests).
    pub fn with_uo_minutes(uo_minutes: u64) -> UOClock {
        UOClock {
            epoch: Instant::now(),
            start_uo_minutes: uo_minutes,
        }
    }

    // -- internal helpers --

    /// Total elapsed UO minutes since epoch.
    fn elapsed_uo_minutes(&self) -> u64 {
        let real_secs = self.epoch.elapsed().as_secs();
        // 1 real minute = 8 UO minutes  =>  1 real second = 8/60 UO minutes
        // To avoid float: uo_minutes = real_secs * 8 / 60
        self.start_uo_minutes + real_secs * 8 / 60
    }

    /// Total UO day count (0-based).
    fn uo_day(&self) -> u64 {
        self.elapsed_uo_minutes() / UO_MINUTES_PER_DAY
    }

    // -- public API --

    /// Current in-game time of day.
    pub fn current_time(&self) -> TimeOfDay {
        let total = self.elapsed_uo_minutes();
        let minute_of_day = (total % UO_MINUTES_PER_DAY) as u16;
        TimeOfDay::new((minute_of_day / 60) as u8, (minute_of_day % 60) as u8)
    }

    /// Current season.
    pub fn current_season(&self) -> Season {
        let season_index = (self.uo_day() / UO_DAYS_PER_SEASON) % 4;
        match season_index {
            0 => Season::Spring,
            1 => Season::Summer,
            2 => Season::Fall,
            _ => Season::Winter,
        }
    }

    /// Phase of the Trammel moon.
    pub fn trammel_moon_phase(&self) -> MoonPhase {
        let phase_index = (self.uo_day() % TRAMMEL_CYCLE_DAYS) as u8;
        MoonPhase::from_index(phase_index)
    }

    /// Phase of the Felucca moon.
    pub fn felucca_moon_phase(&self) -> MoonPhase {
        let phase_index = (self.uo_day() % FELUCCA_CYCLE_DAYS) as u8;
        MoonPhase::from_index(phase_index)
    }

    /// Convenience: both moon phases as (trammel, felucca).
    pub fn moon_phases(&self) -> (MoonPhase, MoonPhase) {
        (self.trammel_moon_phase(), self.felucca_moon_phase())
    }
}

// ---------------------------------------------------------------------------
// WeatherRegion
// ---------------------------------------------------------------------------

/// A rectangular region on the world map that shares the same weather.
#[derive(Debug, Clone)]
pub struct WeatherRegion {
    pub name: String,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub weather: WeatherType,
    /// Temperature modifier (affects snow vs rain in winter, etc.).
    pub temperature: i8,
    /// Intensity of the current weather effect (0..100).
    pub intensity: u8,
}

impl WeatherRegion {
    pub fn new(name: &str, x: u16, y: u16, width: u16, height: u16) -> WeatherRegion {
        WeatherRegion {
            name: name.to_string(),
            x,
            y,
            width,
            height,
            weather: WeatherType::Clear,
            temperature: 20,
            intensity: 0,
        }
    }

    /// Check whether a world coordinate falls inside this region.
    pub fn contains(&self, px: u16, py: u16) -> bool {
        px >= self.x
            && px < self.x.saturating_add(self.width)
            && py >= self.y
            && py < self.y.saturating_add(self.height)
    }
}

// ---------------------------------------------------------------------------
// WeatherSystem
// ---------------------------------------------------------------------------

/// Top-level system that owns the clock and all weather regions.
pub struct WeatherSystem {
    pub clock: UOClock,
    pub regions: Vec<WeatherRegion>,
}

impl WeatherSystem {
    pub fn new() -> WeatherSystem {
        WeatherSystem {
            clock: UOClock::new(),
            regions: Vec::new(),
        }
    }

    pub fn add_region(&mut self, region: WeatherRegion) {
        self.regions.push(region);
    }

    /// Find the first region that contains the given point (if any).
    pub fn region_at(&self, x: u16, y: u16) -> Option<&WeatherRegion> {
        self.regions.iter().find(|r| r.contains(x, y))
    }

    /// Current light level (0=bright .. 30=dark), considering time of day.
    pub fn light_level(&self) -> u8 {
        calculate_light_level(&self.clock.current_time())
    }
}

// ---------------------------------------------------------------------------
// Packets
// ---------------------------------------------------------------------------

/// Build a Weather packet (0x65) — 4 bytes.
///
/// | Byte | Meaning                                            |
/// |------|----------------------------------------------------|
/// | 0    | 0x65 — packet id                                   |
/// | 1    | weather type (0=rain, 1=storm/lightning, 0xFF=none) |
/// | 2    | effect count / intensity (0..70)                    |
/// | 3    | temperature (signed)                                |
pub fn weather_packet(weather: WeatherType, intensity: u8, temperature: i8) -> [u8; 4] {
    let weather_byte: u8 = match weather {
        WeatherType::Rain => 0x00,
        WeatherType::Storm => 0x01,
        WeatherType::Snow => 0x02,
        _ => 0xFF, // Clear / Cloudy / Fog — no particle effect
    };

    // Clamp intensity to the 0..70 range the client expects.
    let clamped_intensity = if weather_byte == 0xFF {
        0
    } else {
        intensity.min(70)
    };

    [0x65, weather_byte, clamped_intensity, temperature as u8]
}

/// Build an Overall Light Level packet (0x4F) — 2 bytes.
///
/// | Byte | Meaning                 |
/// |------|-------------------------|
/// | 0    | 0x4F — packet id        |
/// | 1    | light level (0..30)     |
pub fn light_level_packet(level: u8) -> [u8; 2] {
    [0x4F, level.min(30)]
}

/// Calculate the ambient light level from the time of day.
///
/// Returns a value in 0..30 where 0 is brightest and 30 is darkest.
///
/// Rough schedule:
///   00:00 - 04:59  => 30  (full dark)
///   05:00 - 06:59  => 30 -> 0  (dawn transition)
///   07:00 - 20:59  => 0   (full bright)
///   21:00 - 21:59  => 0 -> 30  (dusk transition)
///   22:00 - 23:59  => 30  (full dark)
pub fn calculate_light_level(time: &TimeOfDay) -> u8 {
    let m = time.total_minutes();
    match time.hour {
        // Full dark: midnight to 04:59
        0..=4 => 30,
        // Dawn: 05:00 (300 min) to 06:59 (419 min) — linearly 30 -> 0
        5..=6 => {
            let dawn_start: u16 = 300; // 05:00
            let dawn_end: u16 = 420; // 07:00 (exclusive)
            let progress = m - dawn_start; // 0..119
            let range = dawn_end - dawn_start; // 120
            (30 - (progress as u32 * 30 / range as u32) as u8).min(30)
        }
        // Full bright: 07:00 to 20:59
        7..=20 => 0,
        // Dusk: 21:00 (1260 min) to 21:59 (1319 min) — linearly 0 -> 30
        21 => {
            let dusk_start: u16 = 1260; // 21:00
            let dusk_end: u16 = 1320; // 22:00 (exclusive)
            let progress = m - dusk_start; // 0..59
            let range = dusk_end - dusk_start; // 60
            (progress as u32 * 30 / range as u32) as u8
        }
        // Full dark: 22:00 to 23:59
        _ => 30,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- TimeOfDay --

    #[test]
    fn time_of_day_is_day() {
        assert!(TimeOfDay::new(12, 0).is_day());
        assert!(TimeOfDay::new(7, 0).is_day());
        assert!(TimeOfDay::new(20, 59).is_day());
        assert!(!TimeOfDay::new(21, 0).is_day());
        assert!(!TimeOfDay::new(3, 0).is_day());
    }

    #[test]
    fn time_of_day_is_night() {
        assert!(TimeOfDay::new(22, 0).is_night());
        assert!(TimeOfDay::new(23, 59).is_night());
        assert!(TimeOfDay::new(0, 0).is_night());
        assert!(TimeOfDay::new(3, 30).is_night());
        assert!(!TimeOfDay::new(5, 0).is_night());
        assert!(!TimeOfDay::new(12, 0).is_night());
    }

    #[test]
    fn time_of_day_is_dawn() {
        assert!(TimeOfDay::new(5, 0).is_dawn());
        assert!(TimeOfDay::new(6, 30).is_dawn());
        assert!(!TimeOfDay::new(4, 59).is_dawn());
        assert!(!TimeOfDay::new(7, 0).is_dawn());
    }

    #[test]
    fn time_of_day_is_dusk() {
        assert!(TimeOfDay::new(21, 0).is_dusk());
        assert!(TimeOfDay::new(21, 30).is_dusk());
        assert!(!TimeOfDay::new(20, 59).is_dusk());
        assert!(!TimeOfDay::new(22, 0).is_dusk());
    }

    #[test]
    fn time_of_day_total_minutes() {
        assert_eq!(TimeOfDay::new(0, 0).total_minutes(), 0);
        assert_eq!(TimeOfDay::new(1, 30).total_minutes(), 90);
        assert_eq!(TimeOfDay::new(23, 59).total_minutes(), 1439);
    }

    #[test]
    fn time_of_day_wraps() {
        let t = TimeOfDay::new(25, 65);
        assert_eq!(t.hour, 1);
        assert_eq!(t.minute, 5);
    }

    // -- MoonPhase --

    #[test]
    fn moon_phase_roundtrip() {
        for i in 0..8 {
            let phase = MoonPhase::from_index(i);
            assert_eq!(phase.to_index(), i);
        }
    }

    #[test]
    fn moon_phase_wraps() {
        assert_eq!(MoonPhase::from_index(8), MoonPhase::NewMoon);
        assert_eq!(MoonPhase::from_index(9), MoonPhase::WaxingCrescent);
    }

    // -- UOClock --

    #[test]
    fn clock_starts_at_midnight() {
        let clock = UOClock::new();
        let time = clock.current_time();
        assert_eq!(time.hour, 0);
        assert_eq!(time.minute, 0);
    }

    #[test]
    fn clock_with_start_time() {
        let clock = UOClock::with_start_time(12, 30);
        let time = clock.current_time();
        assert_eq!(time.hour, 12);
        assert_eq!(time.minute, 30);
    }

    #[test]
    fn clock_season_cycle() {
        // Day 0 => Spring, day 12 => Summer, day 24 => Fall, day 36 => Winter, day 48 => Spring
        let minutes_per_day = UO_MINUTES_PER_DAY;

        let spring = UOClock::with_uo_minutes(0);
        assert_eq!(spring.current_season(), Season::Spring);

        let summer = UOClock::with_uo_minutes(12 * minutes_per_day);
        assert_eq!(summer.current_season(), Season::Summer);

        let fall = UOClock::with_uo_minutes(24 * minutes_per_day);
        assert_eq!(fall.current_season(), Season::Fall);

        let winter = UOClock::with_uo_minutes(36 * minutes_per_day);
        assert_eq!(winter.current_season(), Season::Winter);

        let spring_again = UOClock::with_uo_minutes(48 * minutes_per_day);
        assert_eq!(spring_again.current_season(), Season::Spring);
    }

    #[test]
    fn clock_trammel_moon_phases() {
        let day_minutes = UO_MINUTES_PER_DAY;
        let clock_day0 = UOClock::with_uo_minutes(0);
        assert_eq!(clock_day0.trammel_moon_phase(), MoonPhase::NewMoon);

        let clock_day4 = UOClock::with_uo_minutes(4 * day_minutes);
        assert_eq!(clock_day4.trammel_moon_phase(), MoonPhase::FullMoon);

        // Cycle wraps at 8 days
        let clock_day8 = UOClock::with_uo_minutes(8 * day_minutes);
        assert_eq!(clock_day8.trammel_moon_phase(), MoonPhase::NewMoon);
    }

    #[test]
    fn clock_felucca_moon_phases() {
        let day_minutes = UO_MINUTES_PER_DAY;
        let clock_day0 = UOClock::with_uo_minutes(0);
        assert_eq!(clock_day0.felucca_moon_phase(), MoonPhase::NewMoon);

        let clock_day5 = UOClock::with_uo_minutes(5 * day_minutes);
        assert_eq!(clock_day5.felucca_moon_phase(), MoonPhase::WaningGibbous);

        // Cycle wraps at 10 days
        let clock_day10 = UOClock::with_uo_minutes(10 * day_minutes);
        assert_eq!(clock_day10.felucca_moon_phase(), MoonPhase::NewMoon);
    }

    // -- WeatherRegion --

    #[test]
    fn region_contains() {
        let region = WeatherRegion::new("Britain", 100, 200, 50, 50);
        assert!(region.contains(100, 200));
        assert!(region.contains(149, 249));
        assert!(!region.contains(150, 250));
        assert!(!region.contains(99, 200));
    }

    // -- WeatherSystem --

    #[test]
    fn weather_system_region_lookup() {
        let mut sys = WeatherSystem::new();
        sys.add_region(WeatherRegion::new("Britain", 100, 100, 200, 200));
        sys.add_region(WeatherRegion::new("Minoc", 400, 400, 100, 100));

        let r = sys.region_at(150, 150);
        assert!(r.is_some());
        assert_eq!(r.unwrap().name, "Britain");

        let r2 = sys.region_at(450, 450);
        assert!(r2.is_some());
        assert_eq!(r2.unwrap().name, "Minoc");

        assert!(sys.region_at(0, 0).is_none());
    }

    // -- Light level --

    #[test]
    fn light_level_midnight_is_dark() {
        assert_eq!(calculate_light_level(&TimeOfDay::new(0, 0)), 30);
    }

    #[test]
    fn light_level_noon_is_bright() {
        assert_eq!(calculate_light_level(&TimeOfDay::new(12, 0)), 0);
    }

    #[test]
    fn light_level_dawn_transitions() {
        // Start of dawn should be dark
        assert_eq!(calculate_light_level(&TimeOfDay::new(5, 0)), 30);
        // End of dawn should be nearly bright
        let level = calculate_light_level(&TimeOfDay::new(6, 59));
        assert!(level <= 1, "Expected nearly bright at 06:59, got {}", level);
    }

    #[test]
    fn light_level_dusk_transitions() {
        // Start of dusk should be nearly bright
        assert_eq!(calculate_light_level(&TimeOfDay::new(21, 0)), 0);
        // End of dusk should be nearly dark
        let level = calculate_light_level(&TimeOfDay::new(21, 59));
        assert!(level >= 28, "Expected nearly dark at 21:59, got {}", level);
    }

    #[test]
    fn light_level_late_night_is_dark() {
        assert_eq!(calculate_light_level(&TimeOfDay::new(23, 0)), 30);
    }

    // -- Packets --

    #[test]
    fn weather_packet_rain() {
        let pkt = weather_packet(WeatherType::Rain, 40, 15);
        assert_eq!(pkt[0], 0x65);
        assert_eq!(pkt[1], 0x00); // rain
        assert_eq!(pkt[2], 40);
        assert_eq!(pkt[3], 15);
    }

    #[test]
    fn weather_packet_storm() {
        let pkt = weather_packet(WeatherType::Storm, 60, -5i8);
        assert_eq!(pkt[0], 0x65);
        assert_eq!(pkt[1], 0x01);
        assert_eq!(pkt[2], 60);
        assert_eq!(pkt[3], (-5i8) as u8);
    }

    #[test]
    fn weather_packet_clear_forces_zero_intensity() {
        let pkt = weather_packet(WeatherType::Clear, 50, 20);
        assert_eq!(pkt[1], 0xFF);
        assert_eq!(pkt[2], 0); // intensity forced to 0
    }

    #[test]
    fn weather_packet_clamps_intensity() {
        let pkt = weather_packet(WeatherType::Rain, 200, 10);
        assert_eq!(pkt[2], 70); // clamped
    }

    #[test]
    fn weather_packet_snow() {
        let pkt = weather_packet(WeatherType::Snow, 30, -10i8);
        assert_eq!(pkt[0], 0x65);
        assert_eq!(pkt[1], 0x02); // snow
        assert_eq!(pkt[2], 30);
        assert_eq!(pkt[3], (-10i8) as u8);
    }

    #[test]
    fn light_level_packet_basic() {
        let pkt = light_level_packet(15);
        assert_eq!(pkt[0], 0x4F);
        assert_eq!(pkt[1], 15);
    }

    #[test]
    fn light_level_packet_clamps() {
        let pkt = light_level_packet(50);
        assert_eq!(pkt[1], 30);
    }
}
