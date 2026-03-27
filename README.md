# Rust UO Server

A performant [Ultima Online](https://en.wikipedia.org/wiki/Ultima_Online) server implementation written in Rust. UO is a fantasy MMORPG originally released in 1997 that still has an active community running free shards on open-source server implementations (most notably [ServUO](https://github.com/ServUO/ServUO) in C#). This project aims to provide a Rust alternative with strong type safety, memory safety, and the performance characteristics Rust is known for.

> **Status:** Active development. ~60 source files, ~25,000 lines of Rust, 870 tests passing.
> A UO client (ClassicUO) can connect, log in, walk around Britain, open a backpack, mount/dismount an ethereal horse, bank at a Banker NPC, fight a Dragon, and PvP with a full ghost/resurrection cycle. World NPCs auto-populate from ServUO spawn data.

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
| `huffman` | Huffman compression for in-game packets |
| `error` | Unified `ServerError` enum with `thiserror` |
| `config` | TOML-based server configuration with sensible defaults |
| `shutdown` | Graceful Ctrl+C shutdown with `ShutdownSignal` (Arc\<AtomicBool\>) |
| `connection` | Client connection state machine (7 states: Connecting → InGame) |
| `connections` | Thread-safe connection manager with in-range broadcast |
| `encryption` | UO login encryption (XOR key rotation) |
| `packet_validation` | Rate limiting, string sanitisation, coordinate bounds checking |
| `packets` | Packet registry with metadata for known UO packet types |

### UO Protocol Packets

| Module | Packets |
|--------|---------|
| `tcp/packets` | Login flow: 0xEF, 0x80, 0xA0, 0x91; server list, redirect, features, character list; 0x78 Mobile Incoming (with equipment), 0x20 Mobile Update, 0x0B Damage, 0x1D Remove, 0x2C Ghost/Resurrect, 0x88 Paperdoll, 0x2E Equip Item |
| `movement` | 0x02 request, 0x20 draw player, 0x21 reject, 0x22 acknowledge |
| `speech` | 0x1C ASCII, 0xAD unicode parse, system messages |
| `status_packets` | 0x11 status bar (type flags 0–4 incl. AOS), 0x3A skill updates |
| `container_packets` | 0x24 open, 0x3C contents, 0x25 add, 0x2E equip, 0x07/0x08 pick up/drop |
| `targeting` | 0x6C target cursor (build + parse) with TargetManager |
| `gump` | 0xB0 display (GumpBuilder fluent API), 0xB1 response parse |
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
| `map_files` | Parser for UO `.mul` files (map terrain, statics, index); handles `.mul` and `.uop` |
| `uop` | UOP LegacyMUL archive parser for newer UO clients |
| `data_files` | Startup loader for UO client data files; `UO_DATA_DIR` env override |
| `spells` | All 64 magery spells across 8 circles with reagents and requirements |
| `crafting` | 10 craft skills, 23 resources, recipes with success/exceptional chance |
| `loot` | Drop tables with rarity tiers, gold ranges |
| `vendor` | 19 vendor types, buy/sell/restock, pre-built shop templates |
| `housing` | 15 house types, security levels, 7-stage decay, lockdowns/secures |
| `resources` | Mining (9 ore tiers), lumberjacking (7 wood), fishing (5 types) |
| `quests` | Quest lifecycle, objectives, prerequisites, chains, time limits |
| `guild` | Guild types, ranks (promote/demote), wars, alliances |
| `party` | Invite/accept/decline, loot sharing, max 10 members |
| `account` | Access levels (Player → Owner), character slots, banning |
| `commands` | GM command registry with pre-built commands, access gating |
| `events` | Thread-safe pub/sub event bus with 18 game event types |
| `persistence` | JSON save/load for game state |
| `world_state` | Thread-safe world state container with dirty-flag auto-save |
| `char_slots` | Per-account character slot management (up to 7 slots), JSON-backed |
| `pathfinding` | A\* pathfinding over `MapData` with passability checks |
| `npc` | Thread-safe NPC registry (`NpcManager`); Dragon and Banker constructors with full stats |
| `spawn_loader` | Parses ServUO `felucca.xml` spawn files; auto-populates `NpcManager` on startup |

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (edition 2021)
- A UO client installation for map data (optional — server runs without it)
- [ServUO-pub57](https://github.com/ServUO/ServUO) source for spawn data (optional)

### Build & Run

```bash
cargo build --release
RUST_LOG=info cargo run --release
```

### Environment Variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `UO_DATA_DIR` | `C:/Program Files (x86)/Electronic Arts/Ultima Online Classic` | UO client data files (map terrain) |
| `SERV_UO_DIR` | `C:/Users/<you>/Downloads/ServUO-pub57/ServUO-pub57` | ServUO source for `Spawns/felucca.xml` |
| `RUST_LOG` | — | Log verbosity (`error`/`warn`/`info`/`debug`/`trace`) |

### Configuration

```toml
# server.toml (all optional)
[network]
bind_address = "127.0.0.1"
port = 2593

[shard]
name = "My Shard"
```

### Run Tests

```bash
cargo test
```

870 tests covering packet construction/parsing, game mechanics, data structures, and edge cases.

## What Works End-to-End

| Feature | Details |
|---------|---------|
| Client connect | TCP accepted, version-gated (≥4.0), login seed parsed |
| Authentication | 0x80 → `AccountManager::authenticate()`; auto-create on first login |
| Character select | 0x5D → full init: 0x1B login confirm, 0x11 status bar, 0x20 draw player, 0x55 complete |
| Movement | 0x02 → passability check → ack or reject; broadcasts 0x20 to nearby players |
| Chat | 0xAD → broadcast 0x1C to in-range players; `[` prefix → GM command routing |
| War mode | 0x72 → `war_mode` flag per connection |
| PvP combat | 0x05 → swing timer → range check → hit/damage formulas → 0x0B → 0x11 status |
| Death / ghost | HP ≤ 0 → 0x2C ghost to dying player |
| Resurrection | 0x06 near Ankh → 10% HP restore → 0x2C living |
| Paperdoll | 0x06 on own serial → 0x88 Open Paperdoll |
| Backpack | 0x06 on backpack serial → 0x24 open container + 0x3C contents |
| Ethereal horse | 0x06 on statuette → mount/dismount (0x2E equip/remove layer 0x19 + 0x78 update) |
| Banker NPC | 0x06 on banker → 0x24 open bank box + 0x3C contents |
| Dragon NPC | 2000 HP, 40/100/20/40/40 resistances, attackable, removed on death |
| NPC visibility | 0x78 sent for all nearby NPCs on player login |
| Spawn data | `spawn_loader` reads `Spawns/felucca.xml` → thousands of world NPCs on startup |
| Persistence | Characters + accounts saved on shutdown and every 5 minutes |

## Progress

### Completed

- [x] Multi-threaded timer system, async TCP server, login/shard packet flow
- [x] Huffman compression, proper error handling, structured logging, graceful shutdown
- [x] Connection state machine, UO login encryption, packet validation and rate limiting
- [x] Movement, speech, status, container, targeting, gump, effect, weather packets
- [x] Character model (58 skills, stats, heal/damage), combat formulas, item/inventory system
- [x] World/map data structures, `.mul`/`.uop` file parsers, `UO_DATA_DIR` env override
- [x] Spell system (64 magery), crafting, loot, vendor, resource gathering, housing
- [x] Guild, party, quest systems; account management; GM command registry; event bus
- [x] World + character persistence (JSON, auto-save every 5 minutes)
- [x] A\* pathfinding over live map data
- [x] Account authentication end-to-end; character linked to TCP session
- [x] In-range broadcast system (Chebyshev); players see each other on login and movement
- [x] PvP combat with war mode, swing timers, death/ghost, Ankh resurrection
- [x] Paperdoll (0x88), backpack (0x24/0x3C), ethereal horse mount/dismount
- [x] Banker NPC — opens player bank box on double-click
- [x] Static NPCs: Dragon (attackable) and Banker at Britain positions
- [x] `spawn_loader` — reads ServUO `felucca.xml`; auto-populates world with thousands of NPCs

### Next Steps

- [ ] Faction system (Order vs Chaos): assignment, notoriety on 0x78, kill tracking, team chat
- [ ] NPC re-visibility when a player walks into range mid-session
- [ ] Death broadcast to nearby players (0x17 ghost body)
- [ ] Skill gain triggered by combat actions

## License

This project is open source. Contributions welcome.
