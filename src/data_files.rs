/// UO client data-file loader.
///
/// Locates and loads the binary files that UO clients ship with (map terrain,
/// statics, statics index).  Both the legacy `.mul` layout and the newer
/// `.uop` LegacyMUL format are supported transparently via `MapData::load_auto`.
///
/// # Typical usage
///
/// ```no_run
/// use rust_uo_server::data_files::DataFiles;
///
/// let df = DataFiles::new("C:/Program Files (x86)/Electronic Arts/Ultima Online Classic");
/// match df.load_felucca() {
///     Ok(map) => log::info!("Felucca loaded: {} bytes of terrain data", map.map_len()),
///     Err(e)  => log::warn!("Could not load Felucca map data: {}", e),
/// }
/// ```

use std::io;
use crate::map_files::MapData;

/// Dimensions (in tiles) for every UO map.
///
/// The `column_stride` is the Y-dimension used by `MapData::load_auto`; it
/// equals `height` because UO stores map blocks in column-major order
/// (Y varies fastest).
#[derive(Debug, Clone, Copy)]
pub struct MapDef {
    pub index: u8,
    pub width: u32,
    pub height: u32,
}

impl MapDef {
    /// The stride passed to `MapData` for block-index calculation.
    ///
    /// UO maps are stored column-major (X outer, Y inner), so the stride is
    /// the number of blocks per column = height / 8.
    pub fn column_stride(&self) -> u32 {
        self.height
    }
}

// Known UO map definitions.
pub const MAP_FELUCCA:  MapDef = MapDef { index: 0, width: 7168, height: 4096 };
pub const MAP_TRAMMEL:  MapDef = MapDef { index: 1, width: 7168, height: 4096 };
pub const MAP_ILSHENAR: MapDef = MapDef { index: 2, width: 2304, height: 1600 };
pub const MAP_MALAS:    MapDef = MapDef { index: 3, width: 2560, height: 2048 };
pub const MAP_TOKUNO:   MapDef = MapDef { index: 4, width: 1448, height: 1448 };
pub const MAP_TERMUR:   MapDef = MapDef { index: 5, width: 1280, height: 4096 };

/// Knows where to find UO client data files and can load them on demand.
#[derive(Debug, Clone)]
pub struct DataFiles {
    /// Root directory containing UO client data files.
    pub data_dir: String,
}

impl DataFiles {
    /// Create a `DataFiles` pointing at the given directory.
    pub fn new(data_dir: impl Into<String>) -> Self {
        Self { data_dir: data_dir.into() }
    }

    fn path(&self, filename: &str) -> String {
        format!("{}/{}", self.data_dir, filename)
    }

    /// Load map terrain and statics for one map definition.
    pub fn load_map(&self, def: MapDef) -> io::Result<MapData> {
        let mul_path    = self.path(&format!("map{}.mul",           def.index));
        let uop_path    = self.path(&format!("map{}LegacyMUL.uop",  def.index));
        let staidx_path = self.path(&format!("staidx{}.mul",        def.index));
        let statics_path= self.path(&format!("statics{}.mul",       def.index));

        MapData::load_auto(&mul_path, &uop_path, &staidx_path, &statics_path, def.column_stride())
    }

    /// Convenience: load Felucca (map 0).
    pub fn load_felucca(&self) -> io::Result<MapData> {
        self.load_map(MAP_FELUCCA)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_stride_is_height() {
        assert_eq!(MAP_FELUCCA.column_stride(), 4096);
        assert_eq!(MAP_TRAMMEL.column_stride(), 4096);
        assert_eq!(MAP_ILSHENAR.column_stride(), 1600);
        assert_eq!(MAP_MALAS.column_stride(), 2048);
        assert_eq!(MAP_TOKUNO.column_stride(), 1448);
        assert_eq!(MAP_TERMUR.column_stride(), 4096);
    }

    #[test]
    fn load_returns_error_for_missing_directory() {
        let df = DataFiles::new("/this/path/does/not/exist");
        let result = df.load_felucca();
        assert!(result.is_err());
    }

    /// Integration test: only runs when the real UO data files are present.
    #[test]
    fn load_felucca_from_real_files() {
        let data_dir = "C:/Program Files (x86)/Electronic Arts/Ultima Online Classic";
        let uop_path = format!("{}/map0LegacyMUL.uop", data_dir);
        if !std::path::Path::new(&uop_path).exists() {
            // Skip if UO client is not installed.
            return;
        }
        let df = DataFiles::new(data_dir);
        let map = df.load_felucca().expect("failed to load Felucca map");
        // Felucca has 768 columns * 512 rows = 393216 blocks * 196 bytes = 77,070,336 bytes minimum
        assert!(map.map_len() > 77_000_000, "map data too small: {} bytes", map.map_len());
        // Spot-check: read the terrain at a known land tile (Britain area ~1600, 1600)
        let cell = map.get_terrain(1600, 1600).expect("failed to read terrain at Britain");
        // The tile should have a valid (non-zero) tile_id
        assert!(cell.tile_id > 0 || cell.z != 0, "terrain tile at Britain looks empty");
    }
}
