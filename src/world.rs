use bitflags::bitflags;
use std::time::Duration;

// ---------------------------------------------------------------------------
// MapId – the six Ultima Online facets
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapId {
    Felucca,
    Trammel,
    Ilshenar,
    Malas,
    Tokuno,
    TerMur,
}

/// Width x Height dimensions of a map in tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapDimensions {
    pub width: u16,
    pub height: u16,
}

impl MapId {
    /// Return the canonical tile-dimensions for the given facet.
    pub fn dimensions(&self) -> MapDimensions {
        match self {
            MapId::Felucca => MapDimensions {
                width: 7168,
                height: 4096,
            },
            MapId::Trammel => MapDimensions {
                width: 7168,
                height: 4096,
            },
            MapId::Ilshenar => MapDimensions {
                width: 2304,
                height: 1600,
            },
            MapId::Malas => MapDimensions {
                width: 2560,
                height: 2048,
            },
            MapId::Tokuno => MapDimensions {
                width: 1448,
                height: 1448,
            },
            MapId::TerMur => MapDimensions {
                width: 1280,
                height: 4096,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// TileFlags – standard UO tile data flags (bitflags)
// ---------------------------------------------------------------------------

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct TileFlags: u64 {
        const BACKGROUND   = 0x0000_0000_0000_0001;
        const WEAPON       = 0x0000_0000_0000_0002;
        const TRANSPARENT  = 0x0000_0000_0000_0004;
        const TRANSLUCENT  = 0x0000_0000_0000_0008;
        const WALL         = 0x0000_0000_0000_0010;
        const DAMAGING     = 0x0000_0000_0000_0020;
        const IMPASSABLE   = 0x0000_0000_0000_0040;
        const WET          = 0x0000_0000_0000_0080;
        const IGNORED      = 0x0000_0000_0000_0100;
        const SURFACE      = 0x0000_0000_0000_0200;
        const BRIDGE       = 0x0000_0000_0000_0400;
        const GENERIC      = 0x0000_0000_0000_0800;
        const WINDOW       = 0x0000_0000_0000_1000;
        const NO_SHOOT     = 0x0000_0000_0000_2000;
        const FOLIAGE      = 0x0000_0000_0000_4000;
        const HOVER_OVER   = 0x0000_0000_0000_8000;
        const ROOF         = 0x0000_0000_0001_0000;
        const DOOR         = 0x0000_0000_0002_0000;
        const STAIR_BACK   = 0x0000_0000_0004_0000;
        const STAIR_RIGHT  = 0x0000_0000_0008_0000;
        const ALPHA_BLEND  = 0x0000_0000_0010_0000;
        const USE_NEW_ART  = 0x0000_0000_0020_0000;
        const MAP          = 0x0000_0000_0040_0000;
        const CONTAINER    = 0x0000_0000_0080_0000;
        const WEARABLE     = 0x0000_0000_0100_0000;
        const LIGHT_SOURCE = 0x0000_0000_0200_0000;
        const ANIMATION    = 0x0000_0000_0400_0000;
        const NO_DIAGONAL  = 0x0000_0000_0800_0000;
        const ARMOR        = 0x0000_0000_1000_0000;
        const MULTI        = 0x0000_0000_2000_0000;
        const NO_HOUSE     = 0x0000_0000_4000_0000;
        const PARTIAL_HUE  = 0x0000_0000_8000_0000;
    }
}

impl Default for TileFlags {
    fn default() -> Self {
        TileFlags::empty()
    }
}

// ---------------------------------------------------------------------------
// Tile
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    pub id: u16,
    pub z: i8,
    pub flags: TileFlags,
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            id: 0,
            z: 0,
            flags: TileFlags::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Position – a point on a map
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: u16,
    pub y: u16,
    pub z: i8,
}

impl Default for Position {
    fn default() -> Self {
        Self { x: 0, y: 0, z: 0 }
    }
}

// ---------------------------------------------------------------------------
// Rect – axis-aligned bounding rectangle (2-D, ignoring z)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Returns `true` when the 2-D point (px, py) lies inside the rectangle
    /// (inclusive of edges).
    pub fn contains_point(&self, px: u16, py: u16) -> bool {
        px >= self.x
            && py >= self.y
            && px < self.x.saturating_add(self.width)
            && py < self.y.saturating_add(self.height)
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// RegionRules – rules that govern a named region
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegionRules {
    pub can_pvp: bool,
    pub can_recall: bool,
    pub can_gate: bool,
    pub housing_allowed: bool,
    pub guarded: bool,
}

impl Default for RegionRules {
    fn default() -> Self {
        Self {
            can_pvp: false,
            can_recall: true,
            can_gate: true,
            housing_allowed: false,
            guarded: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Region
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    pub name: String,
    pub bounds: Rect,
    pub rules: RegionRules,
}

impl Region {
    pub fn new(name: impl Into<String>, bounds: Rect, rules: RegionRules) -> Self {
        Self {
            name: name.into(),
            bounds,
            rules,
        }
    }

    /// Returns `true` when the given `Position` falls inside this region's
    /// bounding rectangle (z is ignored, as regions are 2-D areas).
    pub fn contains(&self, position: &Position) -> bool {
        self.bounds.contains_point(position.x, position.y)
    }
}

impl Default for Region {
    fn default() -> Self {
        Self {
            name: String::new(),
            bounds: Rect::default(),
            rules: RegionRules::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// SpawnPoint
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnPoint {
    pub position: Position,
    pub mobile_type: String,
    pub range: u16,
    pub count: u16,
    pub min_delay: Duration,
    pub max_delay: Duration,
}

impl SpawnPoint {
    pub fn new(
        position: Position,
        mobile_type: impl Into<String>,
        range: u16,
        count: u16,
        min_delay: Duration,
        max_delay: Duration,
    ) -> Self {
        Self {
            position,
            mobile_type: mobile_type.into(),
            range,
            count,
            min_delay,
            max_delay,
        }
    }
}

impl Default for SpawnPoint {
    fn default() -> Self {
        Self {
            position: Position::default(),
            mobile_type: String::new(),
            range: 0,
            count: 0,
            min_delay: Duration::from_secs(300),
            max_delay: Duration::from_secs(600),
        }
    }
}

// ---------------------------------------------------------------------------
// World – top-level container for a single facet
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct World {
    pub map_id: MapId,
    pub regions: Vec<Region>,
    pub spawn_points: Vec<SpawnPoint>,
}

impl World {
    /// Create a new, empty world for the given facet.
    pub fn new(map_id: MapId) -> Self {
        Self {
            map_id,
            regions: Vec::new(),
            spawn_points: Vec::new(),
        }
    }

    /// Convenience: return the map dimensions for this world's facet.
    pub fn dimensions(&self) -> MapDimensions {
        self.map_id.dimensions()
    }

    /// Add a region to the world.
    pub fn add_region(&mut self, region: Region) {
        self.regions.push(region);
    }

    /// Add a spawn point to the world.
    pub fn add_spawn_point(&mut self, spawn: SpawnPoint) {
        self.spawn_points.push(spawn);
    }

    /// Return all regions that contain the given position.
    pub fn regions_at(&self, position: &Position) -> Vec<&Region> {
        self.regions
            .iter()
            .filter(|r| r.contains(position))
            .collect()
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new(MapId::Felucca)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- MapId ---------------------------------------------------------------

    #[test]
    fn felucca_dimensions() {
        let d = MapId::Felucca.dimensions();
        assert_eq!(d.width, 7168);
        assert_eq!(d.height, 4096);
    }

    #[test]
    fn trammel_dimensions() {
        let d = MapId::Trammel.dimensions();
        assert_eq!(d.width, 7168);
        assert_eq!(d.height, 4096);
    }

    #[test]
    fn ilshenar_dimensions() {
        let d = MapId::Ilshenar.dimensions();
        assert_eq!(d.width, 2304);
        assert_eq!(d.height, 1600);
    }

    #[test]
    fn malas_dimensions() {
        let d = MapId::Malas.dimensions();
        assert_eq!(d.width, 2560);
        assert_eq!(d.height, 2048);
    }

    #[test]
    fn tokuno_dimensions() {
        let d = MapId::Tokuno.dimensions();
        assert_eq!(d.width, 1448);
        assert_eq!(d.height, 1448);
    }

    #[test]
    fn ter_mur_dimensions() {
        let d = MapId::TerMur.dimensions();
        assert_eq!(d.width, 1280);
        assert_eq!(d.height, 4096);
    }

    #[test]
    fn felucca_and_trammel_share_dimensions() {
        assert_eq!(
            MapId::Felucca.dimensions(),
            MapId::Trammel.dimensions()
        );
    }

    // -- TileFlags -----------------------------------------------------------

    #[test]
    fn tile_flags_default_is_empty() {
        assert!(TileFlags::default().is_empty());
    }

    #[test]
    fn tile_flags_combine() {
        let flags = TileFlags::IMPASSABLE | TileFlags::WET;
        assert!(flags.contains(TileFlags::IMPASSABLE));
        assert!(flags.contains(TileFlags::WET));
        assert!(!flags.contains(TileFlags::WALL));
    }

    #[test]
    fn tile_flags_intersection() {
        let a = TileFlags::WALL | TileFlags::DAMAGING | TileFlags::SURFACE;
        let b = TileFlags::WALL | TileFlags::SURFACE;
        let both = a & b;
        assert!(both.contains(TileFlags::WALL));
        assert!(both.contains(TileFlags::SURFACE));
        assert!(!both.contains(TileFlags::DAMAGING));
    }

    #[test]
    fn tile_flags_removal() {
        let mut flags = TileFlags::IMPASSABLE | TileFlags::WALL;
        flags.remove(TileFlags::WALL);
        assert!(flags.contains(TileFlags::IMPASSABLE));
        assert!(!flags.contains(TileFlags::WALL));
    }

    #[test]
    fn tile_flags_all_defined_flags_are_distinct() {
        // Verify none of the defined flags share a bit.
        let all = [
            TileFlags::BACKGROUND,
            TileFlags::WEAPON,
            TileFlags::TRANSPARENT,
            TileFlags::TRANSLUCENT,
            TileFlags::WALL,
            TileFlags::DAMAGING,
            TileFlags::IMPASSABLE,
            TileFlags::WET,
            TileFlags::IGNORED,
            TileFlags::SURFACE,
            TileFlags::BRIDGE,
            TileFlags::GENERIC,
            TileFlags::WINDOW,
            TileFlags::NO_SHOOT,
            TileFlags::FOLIAGE,
            TileFlags::HOVER_OVER,
            TileFlags::ROOF,
            TileFlags::DOOR,
            TileFlags::STAIR_BACK,
            TileFlags::STAIR_RIGHT,
            TileFlags::ALPHA_BLEND,
            TileFlags::USE_NEW_ART,
            TileFlags::MAP,
            TileFlags::CONTAINER,
            TileFlags::WEARABLE,
            TileFlags::LIGHT_SOURCE,
            TileFlags::ANIMATION,
            TileFlags::NO_DIAGONAL,
            TileFlags::ARMOR,
            TileFlags::MULTI,
            TileFlags::NO_HOUSE,
            TileFlags::PARTIAL_HUE,
        ];
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                if i != j {
                    assert!(
                        (*a & *b).is_empty(),
                        "Flags at index {} and {} overlap",
                        i,
                        j
                    );
                }
            }
        }
    }

    // -- Tile ----------------------------------------------------------------

    #[test]
    fn tile_default() {
        let t = Tile::default();
        assert_eq!(t.id, 0);
        assert_eq!(t.z, 0);
        assert!(t.flags.is_empty());
    }

    #[test]
    fn tile_with_values() {
        let t = Tile {
            id: 0x1234,
            z: -15,
            flags: TileFlags::SURFACE | TileFlags::BRIDGE,
        };
        assert_eq!(t.id, 0x1234);
        assert_eq!(t.z, -15);
        assert!(t.flags.contains(TileFlags::SURFACE));
        assert!(t.flags.contains(TileFlags::BRIDGE));
    }

    // -- Position ------------------------------------------------------------

    #[test]
    fn position_default() {
        let p = Position::default();
        assert_eq!(p.x, 0);
        assert_eq!(p.y, 0);
        assert_eq!(p.z, 0);
    }

    // -- Rect ----------------------------------------------------------------

    #[test]
    fn rect_contains_point_inside() {
        let r = Rect::new(100, 200, 50, 30);
        assert!(r.contains_point(100, 200)); // top-left corner
        assert!(r.contains_point(125, 215)); // centre
        assert!(r.contains_point(149, 229)); // bottom-right edge (exclusive boundary -1)
    }

    #[test]
    fn rect_excludes_point_outside() {
        let r = Rect::new(100, 200, 50, 30);
        assert!(!r.contains_point(99, 200)); // one left of x
        assert!(!r.contains_point(100, 199)); // one above y
        assert!(!r.contains_point(150, 200)); // one right of the rect
        assert!(!r.contains_point(100, 230)); // one below the rect
    }

    #[test]
    fn rect_default_contains_nothing() {
        let r = Rect::default();
        // width/height are 0, so nothing can be inside.
        assert!(!r.contains_point(0, 0));
    }

    // -- RegionRules ---------------------------------------------------------

    #[test]
    fn region_rules_default() {
        let rr = RegionRules::default();
        assert!(!rr.can_pvp);
        assert!(rr.can_recall);
        assert!(rr.can_gate);
        assert!(!rr.housing_allowed);
        assert!(!rr.guarded);
    }

    // -- Region --------------------------------------------------------------

    #[test]
    fn region_contains_position() {
        let region = Region::new(
            "Britain",
            Rect::new(1400, 1600, 200, 200),
            RegionRules {
                guarded: true,
                ..RegionRules::default()
            },
        );

        let inside = Position {
            x: 1500,
            y: 1700,
            z: 0,
        };
        let outside = Position {
            x: 100,
            y: 100,
            z: 0,
        };

        assert!(region.contains(&inside));
        assert!(!region.contains(&outside));
    }

    #[test]
    fn region_contains_ignores_z() {
        let region = Region::new(
            "Test",
            Rect::new(0, 0, 10, 10),
            RegionRules::default(),
        );
        let pos_low = Position { x: 5, y: 5, z: -128 };
        let pos_high = Position { x: 5, y: 5, z: 127 };
        assert!(region.contains(&pos_low));
        assert!(region.contains(&pos_high));
    }

    #[test]
    fn region_default() {
        let r = Region::default();
        assert!(r.name.is_empty());
        assert_eq!(r.bounds, Rect::default());
        assert_eq!(r.rules, RegionRules::default());
    }

    // -- SpawnPoint ----------------------------------------------------------

    #[test]
    fn spawn_point_new() {
        let sp = SpawnPoint::new(
            Position { x: 10, y: 20, z: 0 },
            "Skeleton",
            5,
            3,
            Duration::from_secs(60),
            Duration::from_secs(120),
        );
        assert_eq!(sp.mobile_type, "Skeleton");
        assert_eq!(sp.range, 5);
        assert_eq!(sp.count, 3);
        assert_eq!(sp.min_delay, Duration::from_secs(60));
        assert_eq!(sp.max_delay, Duration::from_secs(120));
    }

    #[test]
    fn spawn_point_default() {
        let sp = SpawnPoint::default();
        assert!(sp.mobile_type.is_empty());
        assert_eq!(sp.range, 0);
        assert_eq!(sp.count, 0);
        assert_eq!(sp.min_delay, Duration::from_secs(300));
        assert_eq!(sp.max_delay, Duration::from_secs(600));
    }

    // -- World ---------------------------------------------------------------

    #[test]
    fn world_new_felucca() {
        let w = World::new(MapId::Felucca);
        assert_eq!(w.map_id, MapId::Felucca);
        assert!(w.regions.is_empty());
        assert!(w.spawn_points.is_empty());
    }

    #[test]
    fn world_default_is_felucca() {
        let w = World::default();
        assert_eq!(w.map_id, MapId::Felucca);
    }

    #[test]
    fn world_dimensions() {
        let w = World::new(MapId::Malas);
        let d = w.dimensions();
        assert_eq!(d.width, 2560);
        assert_eq!(d.height, 2048);
    }

    #[test]
    fn world_add_and_query_regions() {
        let mut w = World::new(MapId::Felucca);
        w.add_region(Region::new(
            "Britain",
            Rect::new(1400, 1600, 200, 200),
            RegionRules {
                guarded: true,
                ..RegionRules::default()
            },
        ));
        w.add_region(Region::new(
            "Wilderness",
            Rect::new(0, 0, 7168, 4096),
            RegionRules {
                can_pvp: true,
                ..RegionRules::default()
            },
        ));

        let pos = Position {
            x: 1500,
            y: 1700,
            z: 0,
        };
        let regions = w.regions_at(&pos);
        assert_eq!(regions.len(), 2);

        let names: Vec<&str> = regions.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"Britain"));
        assert!(names.contains(&"Wilderness"));
    }

    #[test]
    fn world_regions_at_returns_empty_when_no_match() {
        let mut w = World::new(MapId::Felucca);
        w.add_region(Region::new(
            "Britain",
            Rect::new(1400, 1600, 200, 200),
            RegionRules::default(),
        ));

        let far_away = Position {
            x: 5000,
            y: 5000,
            z: 0,
        };
        assert!(w.regions_at(&far_away).is_empty());
    }

    #[test]
    fn world_add_spawn_point() {
        let mut w = World::new(MapId::Trammel);
        w.add_spawn_point(SpawnPoint::new(
            Position { x: 100, y: 200, z: 0 },
            "Dragon",
            10,
            1,
            Duration::from_secs(600),
            Duration::from_secs(1200),
        ));
        assert_eq!(w.spawn_points.len(), 1);
        assert_eq!(w.spawn_points[0].mobile_type, "Dragon");
    }
}
