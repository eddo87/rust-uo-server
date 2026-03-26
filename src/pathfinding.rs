/// A* pathfinder using UO map terrain data.
///
/// Finds shortest cardinal-direction (N/S/E/W) paths between two positions
/// by consulting `MapData::is_passable` for each candidate step.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use crate::map_files::MapData;
use crate::movement::{Direction, Position};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// If the Manhattan distance from start to goal exceeds this, return `TooFar`.
const MAX_PATH_DISTANCE: u32 = 48;

/// Maximum number of steps in the returned path.
const MAX_STEPS: usize = 64;

/// Maximum number of nodes the open set may expand before giving up.
const MAX_NODES: usize = 1000;

// ---------------------------------------------------------------------------
// PathfindError
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathfindError {
    /// No path exists between start and goal.
    NoPath,
    /// The start tile is itself impassable.
    StartBlocked,
    /// The destination tile is itself impassable.
    DestinationBlocked,
    /// The Manhattan distance from start to goal exceeds `MAX_PATH_DISTANCE`.
    TooFar,
}

// ---------------------------------------------------------------------------
// Pathfinder
// ---------------------------------------------------------------------------

pub struct Pathfinder<'a> {
    map: &'a MapData,
}

impl<'a> Pathfinder<'a> {
    pub fn new(map: &'a MapData) -> Self {
        Self { map }
    }

    /// Find a path from `start` to `goal` using A*.
    ///
    /// Returns an ordered `Vec<Direction>` of moves, or a `PathfindError`.
    /// The path is at most `MAX_STEPS` moves long, and the search explores at
    /// most `MAX_NODES` nodes before giving up.
    pub fn find_path(
        &self,
        start: Position,
        goal: Position,
    ) -> Result<Vec<Direction>, PathfindError> {
        // ── Trivial case ────────────────────────────────────────────────────
        if start == goal {
            return Ok(Vec::new());
        }

        // ── Distance guard ───────────────────────────────────────────────────
        let dx = (goal.x as i32 - start.x as i32).unsigned_abs();
        let dy = (goal.y as i32 - start.y as i32).unsigned_abs();
        if dx + dy > MAX_PATH_DISTANCE {
            return Err(PathfindError::TooFar);
        }

        // ── Passability pre-checks ───────────────────────────────────────────
        if !self
            .map
            .is_passable(start.x as u32, start.y as u32, start.z)
            .unwrap_or(false)
        {
            return Err(PathfindError::StartBlocked);
        }

        if !self
            .map
            .is_passable(goal.x as u32, goal.y as u32, goal.z)
            .unwrap_or(false)
        {
            return Err(PathfindError::DestinationBlocked);
        }

        // ── A* ───────────────────────────────────────────────────────────────
        // Open set: (f_cost, g_cost, x, y)
        // We use `Reverse` so that BinaryHeap acts as a min-heap.
        let mut open: BinaryHeap<Reverse<(u32, u32, u16, u16)>> = BinaryHeap::new();

        // g_cost for each visited node.
        let mut g_score: HashMap<(u16, u16), u32> = HashMap::new();

        // came_from[(x, y)] = (parent_x, parent_y)
        let mut came_from: HashMap<(u16, u16), (u16, u16)> = HashMap::new();

        let h_start = manhattan(start.x, start.y, goal.x, goal.y);
        open.push(Reverse((h_start, 0, start.x, start.y)));
        g_score.insert((start.x, start.y), 0);

        let mut nodes_explored: usize = 0;

        while let Some(Reverse((_f, g, cx, cy))) = open.pop() {
            // Reached goal?
            if cx == goal.x && cy == goal.y {
                return Ok(reconstruct_path(&came_from, (start.x, start.y), (cx, cy)));
            }

            // Abandon search if we have exceeded the node budget.
            nodes_explored += 1;
            if nodes_explored > MAX_NODES {
                return Err(PathfindError::NoPath);
            }

            // Skip if we have already found a cheaper route to this node
            // (stale entry in the heap).
            let best_g = *g_score.get(&(cx, cy)).unwrap_or(&u32::MAX);
            if g > best_g {
                continue;
            }

            // The new g for all neighbours is g + 1 (uniform move cost).
            let ng = g + 1;
            if ng as usize > MAX_STEPS {
                continue;
            }

            let cur_pos = Position { x: cx, y: cy, z: start.z };

            for dir in &[
                Direction::North,
                Direction::South,
                Direction::East,
                Direction::West,
            ] {
                let neighbor = match Self::step(cur_pos, *dir) {
                    Some(p) => p,
                    None => continue,
                };

                // Ask the map whether this tile is passable.
                let passable = self
                    .map
                    .is_passable(neighbor.x as u32, neighbor.y as u32, neighbor.z)
                    .unwrap_or(false);
                if !passable {
                    continue;
                }

                let key = (neighbor.x, neighbor.y);
                let prev_g = *g_score.get(&key).unwrap_or(&u32::MAX);
                if ng < prev_g {
                    g_score.insert(key, ng);
                    came_from.insert(key, (cx, cy));
                    let h = manhattan(neighbor.x, neighbor.y, goal.x, goal.y);
                    open.push(Reverse((ng + h, ng, neighbor.x, neighbor.y)));
                }
            }
        }

        Err(PathfindError::NoPath)
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    /// Attempt one step from `pos` in direction `dir`.
    ///
    /// Returns `None` if the resulting coordinates would underflow (u16 wrap).
    fn step(pos: Position, dir: Direction) -> Option<Position> {
        let (nx, ny) = match dir {
            Direction::North => (pos.x, pos.y.checked_sub(1)?),
            Direction::South => (pos.x, pos.y.checked_add(1)?),
            Direction::East => (pos.x.checked_add(1)?, pos.y),
            Direction::West => (pos.x.checked_sub(1)?, pos.y),
            // Diagonal directions are not used in 4-directional pathfinding.
            Direction::Northeast => (pos.x.checked_add(1)?, pos.y.checked_sub(1)?),
            Direction::Southeast => (pos.x.checked_add(1)?, pos.y.checked_add(1)?),
            Direction::Southwest => (pos.x.checked_sub(1)?, pos.y.checked_add(1)?),
            Direction::Northwest => (pos.x.checked_sub(1)?, pos.y.checked_sub(1)?),
        };
        Some(Position { x: nx, y: ny, z: pos.z })
    }

    /// Return the cardinal direction from one tile to an adjacent tile.
    ///
    /// Assumes `from` and `to` are exactly one step apart along an axis.
    fn direction_between(from: (u16, u16), to: (u16, u16)) -> Direction {
        let (fx, fy) = from;
        let (tx, ty) = to;
        if tx > fx {
            Direction::East
        } else if tx < fx {
            Direction::West
        } else if ty > fy {
            Direction::South
        } else {
            Direction::North
        }
    }
}

// ---------------------------------------------------------------------------
// Stand-alone helpers
// ---------------------------------------------------------------------------

/// Manhattan distance between two tile coordinates.
fn manhattan(x1: u16, y1: u16, x2: u16, y2: u16) -> u32 {
    (x1 as i32 - x2 as i32).unsigned_abs() + (y1 as i32 - y2 as i32).unsigned_abs()
}

/// Walk the `came_from` map backwards from `goal` to `start`, then reverse
/// the list to produce an ordered sequence of `Direction`s.
fn reconstruct_path(
    came_from: &HashMap<(u16, u16), (u16, u16)>,
    start: (u16, u16),
    goal: (u16, u16),
) -> Vec<Direction> {
    let mut path = Vec::new();
    let mut current = goal;

    while current != start {
        let parent = match came_from.get(&current) {
            Some(&p) => p,
            None => break,
        };
        path.push(Pathfinder::direction_between(parent, current));
        current = parent;
    }

    path.reverse();
    path
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_files::{MapData, MAP_BLOCK_SIZE, INDEX_ENTRY_SIZE};

    // ── Test-data builders ──────────────────────────────────────────────────

    fn write_u16_le(buf: &mut Vec<u8>, v: u16) {
        buf.extend_from_slice(&v.to_le_bytes());
    }

    fn write_u32_le(buf: &mut Vec<u8>, v: u32) {
        buf.extend_from_slice(&v.to_le_bytes());
    }

    fn write_i8(buf: &mut Vec<u8>, v: i8) {
        buf.push(v as u8);
    }

    /// One 196-byte block where every cell has `tile_id=1, z=0` (passable).
    fn passable_block() -> Vec<u8> {
        let mut buf = Vec::with_capacity(MAP_BLOCK_SIZE);
        write_u32_le(&mut buf, 0); // header
        for _ in 0..64 {
            write_u16_le(&mut buf, 1); // tile_id > 0
            write_i8(&mut buf, 0);     // z = 0
        }
        assert_eq!(buf.len(), MAP_BLOCK_SIZE);
        buf
    }

    /// One 196-byte block with a partial wall: cells at column `col_wall` AND
    /// rows `wall_row_start..wall_row_end` (exclusive) have z=50 (impassable);
    /// all other cells have z=0 (passable).
    ///
    /// "column" is the x-local index (= world-x for block 0).
    /// "row"    is the y-local index (= world-y for block 0).
    fn block_with_partial_wall(col_wall: usize, wall_row_start: usize, wall_row_end: usize) -> Vec<u8> {
        let mut buf = Vec::with_capacity(MAP_BLOCK_SIZE);
        write_u32_le(&mut buf, 0);
        for row in 0..8usize {
            for col in 0..8usize {
                write_u16_le(&mut buf, 1);
                if col == col_wall && row >= wall_row_start && row < wall_row_end {
                    write_i8(&mut buf, 50); // impassable
                } else {
                    write_i8(&mut buf, 0); // passable
                }
            }
        }
        assert_eq!(buf.len(), MAP_BLOCK_SIZE);
        buf
    }

    /// "No statics" index entry (12 bytes, offset = 0xFFFFFFFF).
    fn no_statics_index() -> Vec<u8> {
        let mut buf = Vec::with_capacity(INDEX_ENTRY_SIZE);
        write_u32_le(&mut buf, 0xFFFF_FFFF);
        write_u32_le(&mut buf, 0);
        write_u32_le(&mut buf, 0);
        buf
    }

    /// Build a `MapData` backed by a single 8x8 block (map_width = 8).
    /// The index contains one "no statics" entry.
    fn single_block_map(block: Vec<u8>) -> MapData {
        let index = no_statics_index();
        MapData::from_raw(block, index, Vec::new(), 8)
    }

    // ── Tests ───────────────────────────────────────────────────────────────

    /// start == goal → empty path, no error.
    #[test]
    fn find_path_same_start_and_goal() {
        let map = single_block_map(passable_block());
        let pf = Pathfinder::new(&map);
        let start = Position { x: 2, y: 2, z: 0 };
        let result = pf.find_path(start, start);
        assert_eq!(result, Ok(vec![]));
    }

    /// One step East: start=(0,0), goal=(1,0).
    #[test]
    fn find_path_adjacent_cells() {
        let map = single_block_map(passable_block());
        let pf = Pathfinder::new(&map);
        let start = Position { x: 0, y: 0, z: 0 };
        let goal  = Position { x: 1, y: 0, z: 0 };
        let path = pf.find_path(start, goal).expect("should find path");
        assert_eq!(path.len(), 1);
        assert_eq!(path[0], Direction::East);
    }

    /// Five steps South (y increases): start=(2,0), goal=(2,5).
    #[test]
    fn find_path_straight_line() {
        let map = single_block_map(passable_block());
        let pf = Pathfinder::new(&map);
        let start = Position { x: 2, y: 0, z: 0 };
        let goal  = Position { x: 2, y: 5, z: 0 };
        let path = pf.find_path(start, goal).expect("should find path");
        assert_eq!(path.len(), 5);
        for dir in &path {
            assert_eq!(*dir, Direction::South);
        }
    }

    /// Wall test: a partial vertical wall of impassable cells at x=4, rows 1–6.
    /// Row 0 (y=0) is open, so the path must detour north, then east, then south.
    ///
    /// Map layout (8×8, x: 0..7, y: 0..7):
    ///   x=4, y=1..=6: z=50 (impassable)
    ///   everywhere else: z=0 (passable)
    ///
    /// start=(3,3), goal=(5,3) — direct East is blocked at x=4, y=3.
    /// Optimal detour: North ×3 to y=0, East ×2 to x=5, South ×3 to y=3 = 8 steps.
    #[test]
    fn find_path_around_wall() {
        // Wall at column 4 (world-x=4), rows 1..7 (world-y=1..=6),
        // leaving row 0 (y=0) open as the bypass.
        let block = block_with_partial_wall(4, 1, 7);
        let map = single_block_map(block);
        let pf = Pathfinder::new(&map);

        let start = Position { x: 3, y: 3, z: 0 };
        let goal  = Position { x: 5, y: 3, z: 0 };

        let path = pf.find_path(start, goal).expect("should find a path around the wall");

        // Minimum detour is 8 steps (3N + 2E + 3S).
        assert!(path.len() >= 8, "expected at least 8 steps, got {}", path.len());
        assert!(path.len() <= MAX_STEPS, "path too long: {}", path.len());

        // Replay the path and confirm we land on the goal.
        let mut pos = start;
        for dir in &path {
            pos = Pathfinder::step(pos, *dir).expect("step returned None during replay");
        }
        assert_eq!((pos.x, pos.y), (goal.x, goal.y), "path did not lead to goal");
    }

    /// Completely surrounded by walls → NoPath.
    ///
    /// We place a single passable cell at (2,2) and make everything else
    /// impassable by using a wall_block with all z=50, then patch (2,2) to
    /// z=0.  The goal (4,4) is impassable → DestinationBlocked.
    ///
    /// Alternatively, make start passable but all 4 neighbours impassable.
    /// We construct a specialised block for that.
    #[test]
    fn find_path_no_path() {
        // Build a block where (2,2) is passable but ALL its neighbours
        // (1,2), (3,2), (2,1), (2,3) are impassable (z=50).
        // Everything else is also impassable.
        let mut buf = Vec::with_capacity(MAP_BLOCK_SIZE);
        write_u32_le(&mut buf, 0);
        for row in 0..8usize {
            for col in 0..8usize {
                write_u16_le(&mut buf, 1);
                // Only world-x=2, world-y=2 is passable (col=x=2, row=y=2).
                if col == 2 && row == 2 {
                    write_i8(&mut buf, 0);
                } else {
                    write_i8(&mut buf, 50);
                }
            }
        }

        let map = single_block_map(buf);
        let pf = Pathfinder::new(&map);

        let start = Position { x: 2, y: 2, z: 0 };
        let goal  = Position { x: 5, y: 5, z: 0 };

        // Goal tile is impassable → DestinationBlocked.
        let err = pf.find_path(start, goal).expect_err("should fail");
        assert_eq!(err, PathfindError::DestinationBlocked);
    }

    /// Goal is 100 tiles away → TooFar.
    #[test]
    fn find_path_too_far() {
        let map = single_block_map(passable_block());
        let pf = Pathfinder::new(&map);
        let start = Position { x: 0,   y: 0, z: 0 };
        let goal  = Position { x: 100, y: 0, z: 0 };
        let err = pf.find_path(start, goal).expect_err("should be TooFar");
        assert_eq!(err, PathfindError::TooFar);
    }
}
