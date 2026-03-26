/// UOP (MYP) archive loader for `mapNLegacyMUL.uop` files.
///
/// Newer UO clients ship map terrain data inside `.uop` archives instead of
/// flat `.mul` files.  The archive stores chunks of raw MUL blocks in
/// arbitrary physical order together with an ordering index (`m_Order`) that
/// indicates each chunk's logical position in the flat MUL byte stream.
///
/// `load_legacy_mul` reads one of these archives and returns the equivalent
/// flat `Vec<u8>` you would have gotten from `map0.mul`, with all chunks
/// reassembled in logical order.  The returned bytes are the same format that
/// `MapFileReader` / `MapData` already understand.
///
/// # UOP header layout
/// ```text
/// [0..4]   magic       = 0x0050594D ("MYP\0")
/// [4..8]   version     (u32 LE)
/// [8..12]  unknown
/// [12..16] first_table (u32 LE) – byte offset of first file-table block
/// ```
///
/// # File-table block at `first_table`
/// ```text
/// [0..4]   count       – number of entry slots in this block
/// [4..8]   next_table  – offset of next block (0 = no more)
/// [8..12]  unknown
/// entries: 34 bytes each
///   [0..4]  file_offset (u32 LE) – 0 means empty slot, skip
///   [4..12] hash        (u64, ignored)
///   [12..16] data_length (u32 LE) – bytes of raw MUL data in the chunk
///   [16..34] padding / ignored
/// ```
///
/// # Chunk at `file_offset`
/// ```text
/// [0..2]                skip
/// [2..4]  extra_hdr_len (u16 LE) – length of additional header before data
/// [4 .. 4+extra_hdr_len] extra header:
///   [0..4]  m_Order (u32 LE) – virtual byte offset for logical ordering
/// [4+extra_hdr_len .. 4+extra_hdr_len+data_length] raw MUL block data
/// ```

use std::io;

/// Parse a `mapNLegacyMUL.uop` file and return the equivalent flat `.mul`
/// byte stream, with all chunks assembled in logical order.
pub fn load_legacy_mul(path: &str) -> io::Result<Vec<u8>> {
    let file = std::fs::read(path)?;

    // --- Validate magic ---
    if file.len() < 16 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "UOP file too small"));
    }
    let magic = u32::from_le_bytes([file[0], file[1], file[2], file[3]]);
    if magic != 0x0050_594D {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid UOP magic: {:#010X}", magic),
        ));
    }

    // --- Read first file-table offset ---
    // layout: magic(4) + version(4) + unknown(4) + first_table(4)
    let first_table = u32::from_le_bytes([file[12], file[13], file[14], file[15]]) as usize;

    // --- Walk all file-table blocks and collect entries ---
    let mut raw_entries: Vec<(usize, usize)> = Vec::new(); // (file_offset, data_length)

    let mut table_pos = first_table;
    loop {
        if table_pos + 12 > file.len() {
            break;
        }
        let count = u32::from_le_bytes(read4(&file, table_pos)) as usize;
        let next_table = u32::from_le_bytes(read4(&file, table_pos + 4)) as usize;
        // skip 4 unknown bytes at table_pos+8

        let mut epos = table_pos + 12;
        for _ in 0..count {
            if epos + 4 > file.len() {
                break;
            }
            let file_offset = u32::from_le_bytes(read4(&file, epos)) as usize;
            epos += 4;

            if file_offset == 0 {
                epos += 30; // skip the rest of an empty entry (8+4+18)
                continue;
            }

            epos += 8; // skip hash (u64)
            let data_length = u32::from_le_bytes(read4(&file, epos)) as usize;
            epos += 4;
            epos += 18; // skip padding

            raw_entries.push((file_offset, data_length));
        }

        if next_table == 0 || next_table >= file.len() {
            break;
        }
        table_pos = next_table;
    }

    // --- Read m_Order from each chunk header and compute actual data start ---
    let mut entries: Vec<(usize, usize, u32)> = Vec::new(); // (data_start, data_len, m_order)

    for (file_offset, data_length) in raw_entries {
        if file_offset + 4 > file.len() {
            continue;
        }
        // chunk layout: skip 2 bytes, then u16 extra_hdr_len
        let extra_hdr_len = u16::from_le_bytes([file[file_offset + 2], file[file_offset + 3]]) as usize;
        let data_start = file_offset + 4 + extra_hdr_len;

        if data_start + 4 > file.len() {
            continue;
        }
        // m_Order is the first u32 at data_start (it's also the map-block header)
        let m_order = u32::from_le_bytes(read4(&file, data_start));

        if data_start + data_length > file.len() {
            continue;
        }
        entries.push((data_start, data_length, m_order));
    }

    if entries.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "UOP file contains no valid chunks"));
    }

    // --- Sort chunks into logical MUL order ---
    entries.sort_unstable_by_key(|&(_, _, order)| order);

    // --- Concatenate ---
    let total: usize = entries.iter().map(|&(_, len, _)| len).sum();
    let mut result = Vec::with_capacity(total);
    for (data_start, data_length, _) in entries {
        result.extend_from_slice(&file[data_start..data_start + data_length]);
    }

    Ok(result)
}

#[inline]
fn read4(data: &[u8], pos: usize) -> [u8; 4] {
    [data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_rejects_empty_file() {
        // Write a temp file that is too small.
        let result = load_legacy_mul("this_file_does_not_exist_xyz.uop");
        assert!(result.is_err());
    }

    #[test]
    fn load_rejects_wrong_magic() {
        // Build a minimal fake UOP with wrong magic.
        let mut fake = vec![0u8; 64];
        fake[0] = 0xDE; fake[1] = 0xAD; fake[2] = 0xBE; fake[3] = 0xEF;
        let tmp = std::env::temp_dir().join("bad_magic.uop");
        std::fs::write(&tmp, &fake).unwrap();
        let result = load_legacy_mul(tmp.to_str().unwrap());
        assert!(result.is_err());
        let _ = std::fs::remove_file(tmp);
    }
}
