# Rust UO Server

A performant [Ultima Online](https://en.wikipedia.org/wiki/Ultima_Online) server implementation written in Rust. UO is a fantasy MMORPG originally released in 1997 that still has an active community running free shards on open-source server implementations (most notably [ServUO](https://github.com/ServUO/ServUO) in C#). This project aims to provide a Rust alternative with strong type safety, memory safety, and the performance characteristics Rust is known for.

> **Status:** Active development. 57 source files, ~22,800 lines of Rust, 841 tests passing.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Main Thread                             │
│  env_logger::init() → timer::start() → tcp::start()            │
│  Ctrl+C → ShutdownSignal → graceful shutdown                   │
└──────────────┬──────────────────────┬───────────────────────────┘
               │                      │
    ┌──────────▼──────────┐  ┌────────▼─────────────────┐
    │   Timer System      │  │   Async TCP Server       │
    │   3 threads:        │  │   async-std on :2593     │
    │   register/         │  │   Connection per client  │
    │   prioritise/       │  │   Packet parse → respond │
    │   execute           │  └──────────────────────────┘
    └─────────────────────┘
```

### Core Infrastructure

| Module | Description |
|--------|-------------|
| `tcp` | Async TCP server with packet parsing, error handling, structured logging |
| `timer` | Multi-threaded timer system (registration, prioritisation, execution) |
| `huffman` | Huffman compression and decompression for in-game packets |
| `error` | Unified `ServerError` enum with `thiserror` (replaces all `unwrap()`) |
| `config` | TOML-based server configuration with sensible defaults |
| `shutdown` | Graceful Ctrl+C shutdown with `ShutdownSignal` (Arc\<AtomicBool\>) |
| `connection` | Client connection state machine (7 states: Connecting → InGame) |
| `connections` | Thread-safe connection manager (Arc\<Mutex\<HashMap\>\>) |
| `encryption` | UO login encryption (XOR key rotation) + Twofish stub |
| `packet_validation` | Rate limiting, string sanitisation, coordinate bounds checking |
| `packets` | Packet registry with metadata for 17 known UO packet types |

### UO Protocol Packets

| Module | Packets |
|--------|---------|
| `tcp/packets` | Login flow: 0xEF, 0x80, 0xA0, 0x91 + server list, redirect, features, character list |
| `movement` | 0x02 request, 0x20 draw player, 0x21 reject, 0x22 acknowledge |
| `speech` | 0x1C ASCII, 0xAD unicode parse, 0xAE unicode send, system messages |
| `status_packets` | 0x11 status bar (type flags 0-4 incl. AOS), 0x3A skill updates |
| `container_packets` | 0x24 open, 0x3C contents, 0x25 add, 0x2E equip, 0x1D remove, 0x07/0x08 pick up/drop |
| `targeting` | 0x6C target cursor (build + parse) with TargetManager |
| `gump` | 0xB0 display (GumpBuilder fluent API), 0xB1 response parse, UTF-16 |
| `effects` | 0x70 graphical, 0x54 sound, 0xC0 extended + spell effect presets |
| `weather` | 0x65 weather, 0x4F light level with dawn/dusk transitions |
| `party` | 0xBF/0x06 party sub-commands (add, remove, message, loot sharing) |

### Game Systems

| Module | Description |
|--------|-------------|
| `character` | Full character model: stats, 58 UO skills, heal/damage/gain mechanics |
| `skills` | Skill gain formulas, caps (700 total, 120 individual), warrior/mage/crafter templates |
| `mobile` | NPC/monster base: AI types, notoriety, spawners with stat ranges |
| `combat` | UO combat formulas: hit chance, damage, armor reduction, swing delay |
| `item` / `inventory` | Equipment layers, item flags, serial generation, recursive weight |
| `world` | 6 UO maps, tile flags (32 bitflags), regions, spawn points |
| `map_files` | Parser for UO `.mul` files (map terrain, statics, index); `MapData::load_auto` handles both `.mul` and `.uop` |
| `uop` | UOP LegacyMUL archive parser — extracts flat map terrain from `mapNLegacyMUL.uop` (newer UO clients) |
| `data_files` | Startup loader: finds and loads all UO client data files; supports `UO_DATA_DIR` env override |
| `spells` | All 64 magery spells across 8 circles with reagents and requirements |
| `crafting` | 10 craft skills, 23 resources, recipes with success/exceptional chance |
| `loot` | Drop tables with rarity tiers, gold ranges (5 pre-built tables) |
| `vendor` | 19 vendor types, buy/sell/restock, 4 pre-built shop templates |
| `housing` | 15 house types, security levels, 7-stage decay, lockdowns/secures |
| `resources` | Mining (9 ore tiers), lumberjacking (7 wood), fishing (5 types) |
| `quests` | Quest lifecycle, objectives, prerequisites, chains, time limits |
| `guild` | Guild types, ranks (promote/demote), wars, alliances |
| `party` | Invite/accept/decline, loot sharing, max 10 members |
| `account` | Access levels (Player → Owner), character slots, banning |
| `commands` | GM command registry with 12 pre-built commands, access gating |
| `events` | Thread-safe pub/sub event bus with 18 game event types |
| `persistence` | JSON save/load for game state |
| `world_state` | Thread-safe world state container with dirty-flag auto-save |
| `char_slots` | Per-account character slot management (up to 7 slots), JSON persistence |
| `pathfinding` | A\* pathfinding over `MapData` with passability checks and 48-tile range limit |

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (edition 2021)

### Build & Run

```bash
cargo build --release
RUST_LOG=info cargo run --release
```

### Configuration

Copy and edit `server.toml` (all settings are optional, defaults are used for missing values):

```toml
[network]
bind_address = "127.0.0.1"
port = 2593

[shard]
name = "My Shard"

[game]
starting_city = "Britain"
```

### Run Tests

```bash
cargo test
```

799 tests covering packet construction/parsing, game mechanics, data structures, and edge cases.

## Log Levels

Control verbosity with the `RUST_LOG` environment variable:

| Level | What you see |
|-------|-------------|
| `error` | Fatal/serious errors only |
| `warn` | Unknown packet IDs, recoverable issues |
| `info` | Server startup, connections, logins |
| `debug` | Packet flow, timer operations, state changes |
| `trace` | Raw hex dumps, passwords, tick-level timer ops |

## Dependencies

| Crate | Purpose |
|-------|---------|
| `async-std` | Async TCP server runtime |
| `chrono` | Timestamp handling for ticks |
| `byteorder` | Big-endian packet serialization |
| `thiserror` | Ergonomic error types |
| `log` + `env_logger` | Structured logging |
| `serde` + `serde_json` + `toml` | Config and persistence serialization |
| `bitflags` | Tile flags, item flags |
| `ctrlc` | Cross-platform Ctrl+C handling |
| `rand` | Combat rolls, loot drops, skill gains |

## Development Journal

The original author has been documenting progress on a [public journal](https://thisdotrob.github.io/) — see posts [tagged "UO server project"](https://thisdotrob.github.io/tag/UO%20server%20project/).

## Progress

### Completed

- [x] Multi-threaded timer system with callbacks
- [x] Async TCP server with non-blocking client handling
- [x] Login/shard selection packet flow
- [x] Huffman compression and decompression
- [x] Proper error handling (zero `unwrap()` in production code)
- [x] Structured logging with configurable levels
- [x] Graceful shutdown with Ctrl+C
- [x] TOML-based server configuration
- [x] Connection state machine
- [x] UO login encryption
- [x] Packet validation and rate limiting
- [x] Movement, speech, status, container, targeting, gump, effect packets
- [x] Character model with 58 skills and stat system
- [x] Combat system with UO formulas
- [x] Item/inventory system with equipment layers
- [x] World/map data structures for all 6 UO maps
- [x] `.mul` map file parser
- [x] Spell system (64 magery spells)
- [x] Crafting, loot, vendor, resource gathering systems
- [x] Housing with decay, security, lockdowns
- [x] Guild, party, quest systems
- [x] Account management with access levels
- [x] GM command system
- [x] Event bus for game mechanics
- [x] Wire game systems into the TCP packet loop (handle in-game packets end-to-end)
- [x] Load UO map/art data files on startup (`.mul` + `.uop` LegacyMUL format)
- [x] World persistence (`WorldState` JSON save/load with dirty-flag auto-save every 5 minutes)
- [x] Character slot persistence (`CharSlots` — 7 slots per account, JSON-backed)
- [x] A\* pathfinding over live map data with passability checks
- [x] Movement validation using map passability (impassable tiles rejected in TCP handler)
- [x] Client version validation (0x82 login deny for clients older than 4.0.0.0)

### Next Steps

- [ ] Database-backed account and world storage

## License

This project is open source. Contributions welcome.
