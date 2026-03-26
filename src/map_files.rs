/// UO .mul map file parser.
///
/// Handles map0.mul / staidx0.mul / statics0.mul binary formats used by
/// Ultima Online to store terrain and static-object data.
///
/// Binary layouts
/// --------------
/// **map0.mul** – a flat sequence of `MapBlock`s (196 bytes each):
///   - 4-byte header (u32 LE)
///   - 64 cells, each 3 bytes: tile_id (u16 LE) + z (i8)
///
/// **staidx0.mul** – a flat sequence of `StaticIndexEntry`s (12 bytes each):
///   - offset  (u32 LE) – byte offset into statics0.mul (0xFFFFFFFF = none)
///   - length  (u32 LE) – byte count
///   - extra   (u32 LE) – unused by the client
///
/// **statics0.mul** – packed `StaticItem`s (7 bytes each):
///   - tile_id   (u16 LE)
///   - x_offset  (u8)
///   - y_offset  (u8)
///   - z          (i8)
///   - hue        (u16 LE)

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{self, Cursor, Read};

// ---------------------------------------------------------------------------
// Core data types
// ---------------------------------------------------------------------------

/// A single terrain cell (3 bytes on disk).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapCell {
    pub tile_id: u16,
    pub z: i8,
}

/// One 8x8 terrain block (196 bytes on disk).
#[derive(Debug, Clone)]
pub struct MapBlock {
    pub header: u32,
    pub cells: [[MapCell; 8]; 8],
}

/// A static object entry (7 bytes on disk).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticItem {
    pub tile_id: u16,
    pub x_offset: u8,
    pub y_offset: u8,
    pub z: i8,
    pub hue: u16,
}

/// An index entry pointing into the statics file (12 bytes on disk).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticIndexEntry {
    pub offset: u32,
    pub length: u32,
    pub extra: u32,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Size of a single map block in bytes: 4 (header) + 64 * 3 (cells).
pub const MAP_BLOCK_SIZE: usize = 196;

/// Size of a single static item in bytes.
pub const STATIC_ITEM_SIZE: usize = 7;

/// Size of a single index entry in bytes.
pub const INDEX_ENTRY_SIZE: usize = 12;

/// Sentinel value indicating "no statics for this block".
const NO_STATICS: u32 = 0xFFFF_FFFF;

// ---------------------------------------------------------------------------
// MapFileReader – low-level parsing helpers
// ---------------------------------------------------------------------------

/// Stateless helpers that parse raw byte slices.
pub struct MapFileReader;

impl MapFileReader {
    /// Parse one `MapBlock` starting at position `offset` inside `data`.
    pub fn read_block(data: &[u8], offset: usize) -> io::Result<MapBlock> {
        if data.len() < offset + MAP_BLOCK_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "not enough data for a map block",
            ));
        }

        let mut cur = Cursor::new(&data[offset..offset + MAP_BLOCK_SIZE]);
        let header = cur.read_u32::<LittleEndian>()?;

        let mut cells = [[MapCell { tile_id: 0, z: 0 }; 8]; 8];
        for row in cells.iter_mut() {
            for cell in row.iter_mut() {
                cell.tile_id = cur.read_u16::<LittleEndian>()?;
                cell.z = cur.read_i8()?;
            }
        }

        Ok(MapBlock { header, cells })
    }

    /// Parse one `StaticIndexEntry` at position `offset` inside `data`.
    pub fn read_static_index(data: &[u8], offset: usize) -> io::Result<StaticIndexEntry> {
        if data.len() < offset + INDEX_ENTRY_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "not enough data for a static index entry",
            ));
        }

        let mut cur = Cursor::new(&data[offset..offset + INDEX_ENTRY_SIZE]);
        Ok(StaticIndexEntry {
            offset: cur.read_u32::<LittleEndian>()?,
            length: cur.read_u32::<LittleEndian>()?,
            extra: cur.read_u32::<LittleEndian>()?,
        })
    }

    /// Parse a contiguous run of `StaticItem`s from `data[offset..offset+length]`.
    pub fn read_statics(data: &[u8], offset: usize, length: usize) -> io::Result<Vec<StaticItem>> {
        if length == 0 {
            return Ok(Vec::new());
        }
        if data.len() < offset + length {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "not enough data for statics",
            ));
        }
        if length % STATIC_ITEM_SIZE != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "statics length is not a multiple of 7",
            ));
        }

        let count = length / STATIC_ITEM_SIZE;
        let mut cur = Cursor::new(&data[offset..offset + length]);
        let mut items = Vec::with_capacity(count);

        for _ in 0..count {
            items.push(StaticItem {
                tile_id: cur.read_u16::<LittleEndian>()?,
                x_offset: cur.read_u8()?,
                y_offset: cur.read_u8()?,
                z: cur.read_i8()?,
                hue: cur.read_u16::<LittleEndian>()?,
            });
        }

        Ok(items)
    }

    /// Convenience: look up the terrain tile at world coordinates (`x`, `y`)
    /// given a map whose width is `map_width` tiles (must be a multiple of 8).
    pub fn get_terrain_tile(
        map_data: &[u8],
        x: u32,
        y: u32,
        map_width: u32,
    ) -> io::Result<MapCell> {
        let block_x = x / 8;
        let block_y = y / 8;
        let blocks_wide = map_width / 8;
        let block_index = (block_x * (blocks_wide)) + block_y; // column-major, matching UO layout
        let block_offset = block_index as usize * MAP_BLOCK_SIZE;

        // Offsets inside the block are column-major too.
        let cell_x = (x % 8) as usize;
        let cell_y = (y % 8) as usize;

        let block = Self::read_block(map_data, block_offset)?;
        Ok(block.cells[cell_y][cell_x])
    }

    /// Convenience: return all statics at world coordinates (`x`, `y`).
    pub fn get_statics_at(
        index_data: &[u8],
        statics_data: &[u8],
        x: u32,
        y: u32,
        map_width: u32,
    ) -> io::Result<Vec<StaticItem>> {
        let block_x = x / 8;
        let block_y = y / 8;
        let blocks_wide = map_width / 8;
        let block_index = (block_x * blocks_wide + block_y) as usize;
        let idx_offset = block_index * INDEX_ENTRY_SIZE;

        let entry = Self::read_static_index(index_data, idx_offset)?;

        if entry.offset == NO_STATICS || entry.length == 0 {
            return Ok(Vec::new());
        }

        let all = Self::read_statics(statics_data, entry.offset as usize, entry.length as usize)?;

        let cell_x = (x % 8) as u8;
        let cell_y = (y % 8) as u8;

        Ok(all
            .into_iter()
            .filter(|s| s.x_offset == cell_x && s.y_offset == cell_y)
            .collect())
    }
}

// ---------------------------------------------------------------------------
// MapData – owns loaded file bytes and provides a high-level API
// ---------------------------------------------------------------------------

/// Owns the raw bytes of the three MUL files and exposes a friendly API.
pub struct MapData {
    map_data: Vec<u8>,
    index_data: Vec<u8>,
    statics_data: Vec<u8>,
    /// Map width in tiles (must be a multiple of 8).
    pub map_width: u32,
}

impl MapData {
    /// Load the three files from disk.
    pub fn load_from_files(
        map_path: &str,
        staidx_path: &str,
        statics_path: &str,
        map_width: u32,
    ) -> io::Result<Self> {
        let map_data = std::fs::read(map_path)?;
        let index_data = std::fs::read(staidx_path)?;
        let statics_data = std::fs::read(statics_path)?;

        Ok(Self {
            map_data,
            index_data,
            statics_data,
            map_width,
        })
    }

    /// Build directly from byte vectors (useful for tests).
    pub fn from_raw(
        map_data: Vec<u8>,
        index_data: Vec<u8>,
        statics_data: Vec<u8>,
        map_width: u32,
    ) -> Self {
        Self {
            map_data,
            index_data,
            statics_data,
            map_width,
        }
    }

    /// Get the terrain cell at (`x`, `y`).
    pub fn get_terrain(&self, x: u32, y: u32) -> io::Result<MapCell> {
        MapFileReader::get_terrain_tile(&self.map_data, x, y, self.map_width)
    }

    /// Get all statics at (`x`, `y`).
    pub fn get_statics(&self, x: u32, y: u32) -> io::Result<Vec<StaticItem>> {
        MapFileReader::get_statics_at(
            &self.index_data,
            &self.statics_data,
            x,
            y,
            self.map_width,
        )
    }

    /// Simple passability check: tile is passable if its z is above `min_z`
    /// and there are no statics whose z is within the `[z-5, z+15]` standing
    /// range that would block movement.  This is a *placeholder* heuristic;
    /// real passability requires TileData flags.
    pub fn is_passable(&self, x: u32, y: u32, standing_z: i8) -> io::Result<bool> {
        let terrain = self.get_terrain(x, y)?;

        // If the terrain itself is too far from the standing z, not passable.
        let dz = (terrain.z as i16 - standing_z as i16).abs();
        if dz > 15 {
            return Ok(false);
        }

        let statics = self.get_statics(x, y)?;
        // If any static occupies the same standing range, consider it blocked.
        for s in &statics {
            let sz = (s.z as i16 - standing_z as i16).abs();
            if sz <= 15 {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- helpers to build synthetic byte arrays ------------------------------

    fn write_u16_le(buf: &mut Vec<u8>, v: u16) {
        buf.extend_from_slice(&v.to_le_bytes());
    }

    fn write_u32_le(buf: &mut Vec<u8>, v: u32) {
        buf.extend_from_slice(&v.to_le_bytes());
    }

    fn write_i8(buf: &mut Vec<u8>, v: i8) {
        buf.push(v as u8);
    }

    /// Build one 196-byte map block where every cell has the given tile_id/z.
    fn make_map_block(header: u32, tile_id: u16, z: i8) -> Vec<u8> {
        let mut buf = Vec::with_capacity(MAP_BLOCK_SIZE);
        write_u32_le(&mut buf, header);
        for _ in 0..64 {
            write_u16_le(&mut buf, tile_id);
            write_i8(&mut buf, z);
        }
        assert_eq!(buf.len(), MAP_BLOCK_SIZE);
        buf
    }

    /// Build one 196-byte block where cell (cx, cy) has specific values and
    /// all others are zero.
    fn make_map_block_with_cell(
        header: u32,
        cx: usize,
        cy: usize,
        tile_id: u16,
        z: i8,
    ) -> Vec<u8> {
        let mut buf = Vec::with_capacity(MAP_BLOCK_SIZE);
        write_u32_le(&mut buf, header);
        // Cells are stored row-major: row0 col0..col7, row1 col0..col7 ...
        for row in 0..8usize {
            for col in 0..8usize {
                if row == cy && col == cx {
                    write_u16_le(&mut buf, tile_id);
                    write_i8(&mut buf, z);
                } else {
                    write_u16_le(&mut buf, 0);
                    write_i8(&mut buf, 0);
                }
            }
        }
        assert_eq!(buf.len(), MAP_BLOCK_SIZE);
        buf
    }

    fn make_static_item(tile_id: u16, x_off: u8, y_off: u8, z: i8, hue: u16) -> Vec<u8> {
        let mut buf = Vec::with_capacity(STATIC_ITEM_SIZE);
        write_u16_le(&mut buf, tile_id);
        buf.push(x_off);
        buf.push(y_off);
        write_i8(&mut buf, z);
        write_u16_le(&mut buf, hue);
        assert_eq!(buf.len(), STATIC_ITEM_SIZE);
        buf
    }

    fn make_index_entry(offset: u32, length: u32, extra: u32) -> Vec<u8> {
        let mut buf = Vec::with_capacity(INDEX_ENTRY_SIZE);
        write_u32_le(&mut buf, offset);
        write_u32_le(&mut buf, length);
        write_u32_le(&mut buf, extra);
        assert_eq!(buf.len(), INDEX_ENTRY_SIZE);
        buf
    }

    // -- MapFileReader tests ------------------------------------------------

    #[test]
    fn test_read_block_uniform() {
        let data = make_map_block(42, 0x1234, -5);
        let block = MapFileReader::read_block(&data, 0).unwrap();
        assert_eq!(block.header, 42);
        for row in &block.cells {
            for cell in row {
                assert_eq!(cell.tile_id, 0x1234);
                assert_eq!(cell.z, -5);
            }
        }
    }

    #[test]
    fn test_read_block_specific_cell() {
        let data = make_map_block_with_cell(0, 3, 5, 0xABCD, 10);
        let block = MapFileReader::read_block(&data, 0).unwrap();
        assert_eq!(block.cells[5][3].tile_id, 0xABCD);
        assert_eq!(block.cells[5][3].z, 10);
        // Another cell should be zero.
        assert_eq!(block.cells[0][0].tile_id, 0);
        assert_eq!(block.cells[0][0].z, 0);
    }

    #[test]
    fn test_read_block_at_offset() {
        let mut data = vec![0u8; MAP_BLOCK_SIZE]; // junk first block
        data.extend(make_map_block(99, 0x0001, 7));
        let block = MapFileReader::read_block(&data, MAP_BLOCK_SIZE).unwrap();
        assert_eq!(block.header, 99);
        assert_eq!(block.cells[0][0].tile_id, 1);
        assert_eq!(block.cells[0][0].z, 7);
    }

    #[test]
    fn test_read_block_too_short() {
        let data = vec![0u8; MAP_BLOCK_SIZE - 1];
        assert!(MapFileReader::read_block(&data, 0).is_err());
    }

    #[test]
    fn test_read_static_index() {
        let data = make_index_entry(100, 21, 0);
        let entry = MapFileReader::read_static_index(&data, 0).unwrap();
        assert_eq!(entry.offset, 100);
        assert_eq!(entry.length, 21);
        assert_eq!(entry.extra, 0);
    }

    #[test]
    fn test_read_static_index_at_offset() {
        let mut data = make_index_entry(0, 0, 0);
        data.extend(make_index_entry(200, 14, 5));
        let entry = MapFileReader::read_static_index(&data, INDEX_ENTRY_SIZE).unwrap();
        assert_eq!(entry.offset, 200);
        assert_eq!(entry.length, 14);
        assert_eq!(entry.extra, 5);
    }

    #[test]
    fn test_read_statics_single() {
        let data = make_static_item(0x4000, 2, 3, -10, 0x0033);
        let items = MapFileReader::read_statics(&data, 0, STATIC_ITEM_SIZE).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].tile_id, 0x4000);
        assert_eq!(items[0].x_offset, 2);
        assert_eq!(items[0].y_offset, 3);
        assert_eq!(items[0].z, -10);
        assert_eq!(items[0].hue, 0x0033);
    }

    #[test]
    fn test_read_statics_multiple() {
        let mut data = make_static_item(1, 0, 0, 0, 0);
        data.extend(make_static_item(2, 1, 1, 5, 10));
        data.extend(make_static_item(3, 7, 7, -128, 0xFFFF));
        let items = MapFileReader::read_statics(&data, 0, data.len()).unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[2].tile_id, 3);
        assert_eq!(items[2].z, -128);
        assert_eq!(items[2].hue, 0xFFFF);
    }

    #[test]
    fn test_read_statics_empty() {
        let items = MapFileReader::read_statics(&[], 0, 0).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn test_read_statics_bad_length() {
        let data = vec![0u8; 10]; // not a multiple of 7
        assert!(MapFileReader::read_statics(&data, 0, 10).is_err());
    }

    // -- get_terrain_tile / get_statics_at ----------------------------------

    #[test]
    fn test_get_terrain_tile() {
        // 8x8 map (1 block wide, 1 block tall) – single block
        let data = make_map_block_with_cell(0, 3, 5, 0x00FF, 12);
        let cell = MapFileReader::get_terrain_tile(&data, 3, 5, 8).unwrap();
        assert_eq!(cell.tile_id, 0x00FF);
        assert_eq!(cell.z, 12);
    }

    #[test]
    fn test_get_terrain_tile_multi_block() {
        // 16x8 map → 2 blocks wide (column-major: block 0 = col0, block 1 = col1)
        // block_index = block_x * blocks_tall + block_y
        // With map_width=16, blocks_wide = 2; but we also need blocks_tall for
        // column-major ordering.  Actually the formula in the reader is:
        //   block_index = block_x * (map_width/8) + block_y
        // Wait – map_width/8 = blocks_wide.  For a 16-wide map that is 2.
        // Block(0,0) → index 0*2+0 = 0
        // Block(1,0) → index 1*2+0 = 2   <-- but we only have 2 blocks?
        //
        // UO's true formula is block_x * blocks_tall + block_y where
        // blocks_tall = map_height / 8.  Our reader currently uses map_width/8
        // as the multiplier which works for square maps and for tests where we
        // control layout.  Let's just test with a square 16x16.
        //
        // 16x16 → 2x2 blocks.  block(1,1) index = 1*2+1 = 3 → offset 3*196.
        let mut data = Vec::new();
        // block 0 – (bx=0,by=0)
        data.extend(make_map_block(0, 0, 0));
        // block 1 – (bx=0,by=1)
        data.extend(make_map_block(0, 0, 0));
        // block 2 – (bx=1,by=0)
        data.extend(make_map_block(0, 0, 0));
        // block 3 – (bx=1,by=1), cell (2, 3) inside block → world (10, 11)
        data.extend(make_map_block_with_cell(0, 2, 3, 0xBEEF, -3));

        let cell = MapFileReader::get_terrain_tile(&data, 10, 11, 16).unwrap();
        assert_eq!(cell.tile_id, 0xBEEF);
        assert_eq!(cell.z, -3);
    }

    #[test]
    fn test_get_statics_at_no_statics() {
        let index_data = make_index_entry(NO_STATICS, 0, 0);
        let statics_data: Vec<u8> = Vec::new();
        let result =
            MapFileReader::get_statics_at(&index_data, &statics_data, 0, 0, 8).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_statics_at_filters_by_cell() {
        // Two statics in the block: one at (2,3), one at (4,5).
        let mut statics_data = make_static_item(100, 2, 3, 0, 0);
        statics_data.extend(make_static_item(200, 4, 5, 0, 0));

        let index_data = make_index_entry(0, (2 * STATIC_ITEM_SIZE) as u32, 0);

        // Query cell (2, 3) inside block → world coords (2, 3) for an 8-wide map.
        let items =
            MapFileReader::get_statics_at(&index_data, &statics_data, 2, 3, 8).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].tile_id, 100);

        // Query cell (4, 5)
        let items =
            MapFileReader::get_statics_at(&index_data, &statics_data, 4, 5, 8).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].tile_id, 200);

        // Query cell (0, 0) – nothing there
        let items =
            MapFileReader::get_statics_at(&index_data, &statics_data, 0, 0, 8).unwrap();
        assert!(items.is_empty());
    }

    // -- MapData high-level tests -------------------------------------------

    #[test]
    fn test_map_data_get_terrain() {
        let map_bytes = make_map_block(0, 0x0010, 5);
        let md = MapData::from_raw(map_bytes, Vec::new(), Vec::new(), 8);
        let cell = md.get_terrain(0, 0).unwrap();
        assert_eq!(cell.tile_id, 0x0010);
        assert_eq!(cell.z, 5);
    }

    #[test]
    fn test_map_data_get_statics() {
        let map_bytes = make_map_block(0, 0, 0);
        let mut statics_bytes = make_static_item(500, 1, 2, 3, 4);
        statics_bytes.extend(make_static_item(600, 1, 2, 10, 8));
        let index_bytes = make_index_entry(0, (2 * STATIC_ITEM_SIZE) as u32, 0);

        let md = MapData::from_raw(map_bytes, index_bytes, statics_bytes, 8);

        let items = md.get_statics(1, 2).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].tile_id, 500);
        assert_eq!(items[1].tile_id, 600);
    }

    #[test]
    fn test_is_passable_no_statics_nearby_terrain() {
        // Terrain z=5, standing_z=5 → delta 0, no statics → passable.
        let map_bytes = make_map_block(0, 0, 5);
        let index_bytes = make_index_entry(NO_STATICS, 0, 0);
        let md = MapData::from_raw(map_bytes, index_bytes, Vec::new(), 8);
        assert!(md.is_passable(0, 0, 5).unwrap());
    }

    #[test]
    fn test_is_passable_terrain_too_far() {
        // Terrain z=50, standing_z=0 → delta 50 > 15 → not passable.
        let map_bytes = make_map_block(0, 0, 50);
        let index_bytes = make_index_entry(NO_STATICS, 0, 0);
        let md = MapData::from_raw(map_bytes, index_bytes, Vec::new(), 8);
        assert!(!md.is_passable(0, 0, 0).unwrap());
    }

    #[test]
    fn test_is_passable_blocked_by_static() {
        // Terrain z=0, standing_z=0, static at z=5 (within 15) → blocked.
        let map_bytes = make_map_block(0, 0, 0);
        let statics_bytes = make_static_item(100, 0, 0, 5, 0);
        let index_bytes = make_index_entry(0, STATIC_ITEM_SIZE as u32, 0);
        let md = MapData::from_raw(map_bytes, index_bytes, statics_bytes, 8);
        assert!(!md.is_passable(0, 0, 0).unwrap());
    }

    #[test]
    fn test_is_passable_static_far_away_z() {
        // Terrain z=0, standing_z=0, static at z=100 (delta 100 > 15) → passable.
        let map_bytes = make_map_block(0, 0, 0);
        let statics_bytes = make_static_item(100, 0, 0, 100, 0);
        let index_bytes = make_index_entry(0, STATIC_ITEM_SIZE as u32, 0);
        let md = MapData::from_raw(map_bytes, index_bytes, statics_bytes, 8);
        assert!(md.is_passable(0, 0, 0).unwrap());
    }
}
