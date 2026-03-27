use std::str;
use std::sync::Arc;
use async_std::{
    channel::{bounded, Receiver},
    net::{TcpListener, TcpStream, ToSocketAddrs},
    prelude::*,
    task,
};
use log::{debug, info, trace, warn};
use std::time::Instant;

use crate::character::Character;
use crate::combat::{ArmorStats, WeaponStats, WeaponType, calculate_swing_delay, resolve_combat_round};
use crate::error::ServerError;
use crate::huffman;
use crate::connections::ConnectionManager;
use crate::connection::ConnectionState;
use crate::map_files::MapData;
use crate::movement::{Direction, MovementRequest, Position};

mod packets;

type Result<T> = std::result::Result<T, ServerError>;

// ---------------------------------------------------------------------------
// Packet length table
// ---------------------------------------------------------------------------

/// Returns the total byte length (including the ID byte) for a given packet.
///
/// Returns `None` for unrecognised packet IDs — callers should drain the
/// accumulator and stop processing when this happens, because without a
/// known length we cannot find the start of the next packet.
fn required_packet_length(id: u8, buf: &[u8]) -> Option<usize> {
    match id {
        0x00 => Some(1),  // null padding: skip
        0x02 => Some(7),  // movement request
        0x05 => Some(5),  // attack request
        0x06 => Some(5),  // double-click (ignored for now)
        0x09 => Some(5),  // single-click (ignored for now)
        0x5D => Some(73), // character select
        0x72 => Some(5),  // war mode (client → server)
        0x73 => Some(2),  // ping
        0x80 => Some(62), // account login request
        0x91 => Some(65), // game server login
        0xA0 => Some(3),  // server select
        0xAD => {         // speech request (variable length)
            if buf.len() < 3 { return None; }
            Some(u16::from_be_bytes([buf[1], buf[2]]) as usize)
        }
        0xEF => Some(21), // encrypted login seed
        _ => None,        // unknown: caller should drain & stop
    }
}

// ---------------------------------------------------------------------------
// Writer task
// ---------------------------------------------------------------------------

async fn write_loop(mut stream: TcpStream, rx: Receiver<Vec<u8>>) {
    while let Ok(packet) = rx.recv().await {
        if stream.write_all(&packet).await.is_err() {
            break;
        }
        let _ = stream.flush().await;
    }
}

// ---------------------------------------------------------------------------
// Connection lifecycle
// ---------------------------------------------------------------------------

async fn connection_loop(
    mut stream: TcpStream,
    connections: ConnectionManager,
    map_data: Arc<Option<MapData>>,
) -> Result<()> {
    let addr = stream.peer_addr()?;
    let conn_id = connections.add_connection(addr);
    info!("Registered connection {} from: {}", conn_id, addr);

    // Per-connection outbound channel — all sends go through here so the
    // writer task can own the write-half of the TCP stream.
    let (tx, rx) = bounded::<Vec<u8>>(64);
    connections.register_sender(conn_id, tx);

    let write_stream = stream.clone();
    task::spawn(write_loop(write_stream, rx));

    // Accumulator-based read loop: append TCP bytes, then dispatch complete
    // packets.  This survives fragmented reads (large packets split across
    // two read() calls, common on non-LAN connections).
    let mut acc: Vec<u8> = Vec::new();
    let mut read_buf = [0u8; 4096];
    loop {
        match stream.read(&mut read_buf).await {
            Ok(0) => {
                info!("Connection {} closed by: {}", conn_id, addr);
                break;
            }
            Ok(n) => {
                acc.extend_from_slice(&read_buf[..n]);
                if let Err(e) = process_accumulator(&mut acc, conn_id, &connections, &map_data).await {
                    warn!("Connection {} packet error: {}", conn_id, e);
                }
            }
            Err(e) => {
                warn!("Connection {} read error: {}", conn_id, e);
                break;
            }
        }
    }

    connections.with_connection_mut(conn_id, |c| {
        let _ = c.transition_to(ConnectionState::Disconnected);
    });
    connections.remove_connection(conn_id);
    Ok(())
}

async fn process_accumulator(
    acc: &mut Vec<u8>,
    conn_id: u64,
    connections: &ConnectionManager,
    map_data: &Arc<Option<MapData>>,
) -> Result<()> {
    loop {
        if acc.is_empty() {
            break;
        }
        let id = acc[0];
        let len = match required_packet_length(id, acc) {
            None => {
                warn!(
                    "Connection {}: unknown packet 0x{:02X}; draining {} bytes",
                    conn_id, id, acc.len()
                );
                acc.clear();
                break;
            }
            Some(l) => l,
        };
        if acc.len() < len {
            break; // wait for more data
        }
        let packet = acc[..len].to_vec();
        acc.drain(..len);
        if let Err(e) = handle_packet(id, &packet, conn_id, connections, map_data).await {
            warn!("Connection {} error handling 0x{:02X}: {}", conn_id, id, e);
        }
    }
    Ok(())
}

async fn accept_loop(
    addr: impl ToSocketAddrs,
    connections: ConnectionManager,
    map_data: Arc<Option<MapData>>,
) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!("TCP listener bound, waiting for connections");
    let mut incoming = listener.incoming();
    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        let addr = stream.peer_addr()?;
        info!("Connection received from: {}", addr);
        let conns = connections.clone();
        let map = map_data.clone();
        task::spawn(connection_loop(stream, conns, map));
    }
    Ok(())
}

pub fn start(connections: ConnectionManager, map_data: Arc<Option<MapData>>) -> Result<()> {
    info!("Starting TCP server on 127.0.0.1:2593");
    task::block_on(accept_loop("127.0.0.1:2593", connections, map_data))
}

// ---------------------------------------------------------------------------
// Static world data
// ---------------------------------------------------------------------------

/// Known Ankh of Sacrifice positions on Felucca.
/// A dead player within 3 tiles of any of these can be resurrected.
const ANKH_POSITIONS: &[(u16, u16)] = &[
    (1457, 1537), // Britain Cemetery
    (1499, 1775), // Britain South Healer
    (547,  1047), // Yew Healer
    (1838, 2820), // Trinsic Healer
    (2726, 693),  // Vesper Healer
    (591,  2245), // Skara Brae Healer
    (4431, 1178), // Moonglow Healer
    (2497, 392),  // Minoc Healer
];

// ---------------------------------------------------------------------------
// Packet handler dispatch
// ---------------------------------------------------------------------------

async fn handle_packet(
    id: u8,
    packet: &[u8],
    conn_id: u64,
    connections: &ConnectionManager,
    map_data: &Arc<Option<MapData>>,
) -> Result<()> {
    // `packet` includes the ID byte; body starts at index 1.
    let body = if packet.len() > 1 { &packet[1..] } else { &[] as &[u8] };

    debug!("Connection {}: handling packet 0x{:02X}", conn_id, id);

    match id {
        // -- Login phase -------------------------------------------------

        0xEF => {
            let mut slice = body;
            let (major, minor, revision, patch) = handle_encrypted_login_seed_packet(&mut slice)?;
            if major < 4 {
                warn!(
                    "Client version {}.{}.{}.{} too old (minimum 4.0.0.0); denying",
                    major, minor, revision, patch
                );
                connections.send_to(conn_id, packets::login_deny_packet(0x06).to_vec());
                return Ok(());
            }
            connections.with_connection_mut(conn_id, |c| {
                c.set_client_version(major, minor, revision, patch);
                let _ = c.transition_to(ConnectionState::LoginSeed);
            });
        }

        0x80 => {
            let mut slice = body;
            let username = handle_account_login_request_packet(&mut slice)?;
            connections.with_connection_mut(conn_id, |c| {
                c.set_account_name(username.clone());
                let _ = c.transition_to(ConnectionState::Authenticating);
                let _ = c.transition_to(ConnectionState::ServerSelect);
            });
            connections.send_to(conn_id, packets::server_list_packet().to_vec());
        }

        0xA0 => {
            let mut slice = body;
            handle_server_select_packet(&mut slice)?;
            connections.with_connection_mut(conn_id, |c| {
                let _ = c.transition_to(ConnectionState::GameLogin);
            });
            connections.send_to(conn_id, packets::server_redirect_packet().to_vec());
        }

        0x91 => {
            let mut slice = body;
            let username = handle_post_login_packet(&mut slice)?;
            connections.with_connection_mut(conn_id, |c| {
                // After a redirect the client reconnects; walk the new
                // connection up to GameLogin before we send features.
                if c.state == ConnectionState::Connecting {
                    let _ = c.transition_to(ConnectionState::LoginSeed);
                    let _ = c.transition_to(ConnectionState::Authenticating);
                    let _ = c.transition_to(ConnectionState::ServerSelect);
                    let _ = c.transition_to(ConnectionState::GameLogin);
                }
                c.set_account_name(username.clone());
            });
            connections.send_to(conn_id, compress_packet(&packets::features_packet()));
            connections.send_to(conn_id, compress_packet(&packets::character_list_packet()));
        }

        0x5D => {
            let mut slice = body;
            let (slot, char_name) = handle_character_select_packet(&mut slice)?;
            let serial = 0x00000001u32 + slot;
            let enter_pos = Position { x: 1496, y: 1628, z: 10 };

            let character = Character::new_default(serial, char_name.clone());
            connections.with_connection_mut(conn_id, |c| {
                c.set_character_name(char_name.clone());
                c.set_position(enter_pos);
                let _ = c.transition_to(ConnectionState::InGame);
                c.character = Some(character.clone());
            });
            info!("Player entering game: {} (serial 0x{:08X})", char_name, serial);

            send_login_init_sequence(conn_id, connections, &character);

            // Tell every nearby in-game player about the arriving player.
            let announce_pkt = packets::mobile_incoming_packet(
                serial, 0x0190, enter_pos.x, enter_pos.y, enter_pos.z,
                0x00, 0x0000, 0x00, 0x03,
            );
            connections.broadcast_to_range(conn_id, enter_pos, 18, compress_packet(&announce_pkt));

            // Send a 0x78 for every already-in-game player that is nearby
            // so the arriving player can see them too.
            let nearby: Vec<(u32, Position)> = connections
                .get_all_connections()
                .into_iter()
                .filter(|c| c.id != conn_id && c.state == ConnectionState::InGame)
                .filter_map(|c| {
                    c.position.and_then(|p| {
                        let dx = (p.x as i32 - enter_pos.x as i32).unsigned_abs() as u16;
                        let dy = (p.y as i32 - enter_pos.y as i32).unsigned_abs() as u16;
                        if dx <= 18 && dy <= 18 { Some((c.serial, p)) } else { None }
                    })
                })
                .collect();
            for (s, p) in nearby {
                let pkt = packets::mobile_incoming_packet(
                    s, 0x0190, p.x, p.y, p.z, 0x00, 0x0000, 0x00, 0x03,
                );
                connections.send_to(conn_id, compress_packet(&pkt));
            }
        }

        // -- In-game ----------------------------------------------------

        0x02 => {
            let mut slice = body;
            if let Some(req) = handle_movement_request(&mut slice)? {
                let current_pos = connections
                    .with_connection_mut(conn_id, |c| c.position)
                    .flatten()
                    .unwrap_or(Position { x: 1496, y: 1628, z: 10 });

                let new_pos = apply_movement(current_pos, req.direction);
                let in_bounds = new_pos.x < 7168 && new_pos.y < 4096;
                let passable = in_bounds
                    && match map_data.as_ref() {
                        Some(map) => map
                            .is_passable(new_pos.x as u32, new_pos.y as u32, new_pos.z)
                            .unwrap_or(true),
                        None => true,
                    };

                if passable {
                    let serial = connections
                        .with_connection_mut(conn_id, |c| {
                            c.set_position(new_pos);
                            c.update_move_sequence(req.sequence_number);
                            c.serial
                        })
                        .unwrap_or(1);

                    let ack = packets::movement_ack_packet(req.sequence_number, 0x01);
                    connections.send_to(conn_id, ack.to_vec());

                    // Let nearby players see the movement.
                    let upd = packets::mobile_update_packet(
                        serial, 0x0190, new_pos.x, new_pos.y, new_pos.z,
                        req.direction.to_byte(), 0x0000, 0x00,
                    );
                    connections.broadcast_to_range(conn_id, new_pos, 18, compress_packet(&upd));

                    debug!(
                        "Movement accepted: seq={}, dir={:?}, pos=({},{})",
                        req.sequence_number, req.direction, new_pos.x, new_pos.y
                    );
                } else {
                    let reject = packets::movement_reject_packet(
                        req.sequence_number, current_pos, req.direction,
                    );
                    connections.send_to(conn_id, reject.to_vec());
                    debug!(
                        "Movement rejected: pos=({},{}) in_bounds={}",
                        new_pos.x, new_pos.y, in_bounds
                    );
                }
            }
        }

        0x05 => {
            // Attack request: [0x05, serial u32 BE]
            if packet.len() < 5 { return Ok(()); }
            let target_serial = u32::from_be_bytes([packet[1], packet[2], packet[3], packet[4]]);

            // Must be alive AND in war mode to attack; record target serial
            let (attacker_alive, atk_war_mode) = connections
                .with_connection_mut(conn_id, |c| {
                    c.target_serial = Some(target_serial);
                    let alive = c.character.as_ref().map(|ch| ch.is_alive).unwrap_or(false);
                    (alive, c.war_mode)
                })
                .unwrap_or((false, false));
            if !attacker_alive || !atk_war_mode { return Ok(()); }

            // Locate target connection
            let Some(target_id) = connections.find_by_serial(target_serial) else {
                return Ok(());
            };

            // Collect attacker combat data (clone to release lock before touching target)
            let attacker_snap = connections.get_connection(conn_id);
            let Some(ref atk) = attacker_snap else { return Ok(()); };
            let Some(ref atk_ch) = atk.character else { return Ok(()); };
            if !atk_ch.is_alive { return Ok(()); }

            // Swing timer: enforce per-character cooldown based on dex/stamina
            let swing_secs = calculate_swing_delay(
                35, // fists speed
                atk_ch.derived_stats.stamina.current,
                atk_ch.stats.dexterity,
            );
            let can_swing = atk.last_swing
                .map(|t| t.elapsed().as_secs_f32() >= swing_secs)
                .unwrap_or(true);
            if !can_swing {
                debug!("Swing timer: conn {} must wait ({:.1}s cooldown)", conn_id, swing_secs);
                return Ok(());
            }
            connections.with_connection_mut(conn_id, |c| c.last_swing = Some(Instant::now()));

            let atk_pos = atk.position.unwrap_or(Position { x: 1496, y: 1628, z: 10 });
            let atk_str = atk_ch.stats.strength;
            let atk_skill = atk_ch.skills
                .get(&crate::character::Skill::Wrestling)
                .map(|e| e.value)
                .unwrap_or(0.0);

            // Collect target position for range check
            let target_snap = connections.get_connection(target_id);
            let Some(ref tgt) = target_snap else { return Ok(()); };
            let Some(ref tgt_ch) = tgt.character else { return Ok(()); };
            if !tgt_ch.is_alive { return Ok(()); }
            let tgt_pos = tgt.position.unwrap_or(Position { x: 1496, y: 1628, z: 10 });
            let tgt_skill = tgt_ch.skills
                .get(&crate::character::Skill::Wrestling)
                .map(|e| e.value)
                .unwrap_or(0.0);
            let tgt_name = tgt_ch.name.clone();

            // Range check: melee requires Chebyshev distance ≤ 2
            let dx = (atk_pos.x as i32 - tgt_pos.x as i32).unsigned_abs() as u16;
            let dy = (atk_pos.y as i32 - tgt_pos.y as i32).unsigned_abs() as u16;
            if dx > 2 || dy > 2 {
                debug!("Attack out of range: {} → serial {}", conn_id, target_serial);
                return Ok(());
            }

            // Resolve combat round (unarmed / fists)
            let weapon = WeaponStats {
                min_damage: 3,
                max_damage: 8,
                speed: 35,
                weapon_type: WeaponType::Fists,
                range: 1,
            };
            let armor = ArmorStats {
                physical_resist: 0,
                fire_resist: 0,
                cold_resist: 0,
                poison_resist: 0,
                energy_resist: 0,
            };
            let result = resolve_combat_round(atk_skill, &weapon, atk_str, tgt_skill, &armor);

            if result.was_hit {
                // Apply damage to target
                let (new_hp, max_hp, target_died) = connections
                    .with_connection_mut(target_id, |c| {
                        if let Some(ref mut ch) = c.character {
                            ch.damage(result.damage_dealt);
                            let hp = ch.derived_stats.hit_points;
                            (hp.current, hp.max, !ch.is_alive)
                        } else {
                            (0, 0, false)
                        }
                    })
                    .unwrap_or((0, 0, false));

                info!(
                    "Combat: conn {} hit {} for {} dmg (HP {}/{})",
                    conn_id, tgt_name, result.damage_dealt, new_hp, max_hp
                );

                // Notify attacker of damage dealt (0x0B)
                let dmg_pkt = packets::damage_notification_packet(target_serial, result.damage_dealt as u16);
                connections.send_to(conn_id, compress_packet(&dmg_pkt));

                // Send updated status bar to target and broadcast to nearby
                if let Some(status_pkt) = build_status_packet_for(target_id, connections) {
                    let compressed = compress_packet(&status_pkt);
                    connections.send_to(target_id, compressed.clone());
                    connections.broadcast_to_range(target_id, tgt_pos, 18, compressed);
                }

                if target_died {
                    info!("Player {} died", tgt_name);
                    connections.send_to(target_id, compress_packet(&packets::ghost_mode_packet()));
                }
            } else {
                debug!("Combat: conn {} missed serial {}", conn_id, target_serial);
            }
        }

        0x06 => {
            // Double-click — resurrect at Ankh when dead, ignore otherwise.
            let (is_dead, pos) = {
                let conn = connections.get_connection(conn_id);
                let dead = conn.as_ref()
                    .and_then(|c| c.character.as_ref())
                    .map(|ch| !ch.is_alive)
                    .unwrap_or(false);
                let pos = conn.as_ref()
                    .and_then(|c| c.position)
                    .unwrap_or(Position { x: 1496, y: 1628, z: 10 });
                (dead, pos)
            };
            if is_dead {
                let near_ankh = ANKH_POSITIONS.iter().any(|&(ax, ay)| {
                    let dx = (pos.x as i32 - ax as i32).unsigned_abs() as u16;
                    let dy = (pos.y as i32 - ay as i32).unsigned_abs() as u16;
                    dx <= 3 && dy <= 3
                });
                if near_ankh {
                    resurrect_player(conn_id, connections);
                } else {
                    let msg = packets::system_message_packet(
                        "You must be near an Ankh of Sacrifice to resurrect.",
                    );
                    connections.send_to(conn_id, compress_packet(&msg));
                }
            }
        }

        0x09 => { /* single-click — ignored */ }

        0x72 => {
            // War mode toggle from client: [war_flag, unk, unk, unk]
            if !body.is_empty() {
                let war = body[0] != 0;
                connections.with_connection_mut(conn_id, |c| c.set_war_mode(war));
                connections.send_to(conn_id, compress_packet(&packets::war_mode_packet(war)));
                debug!("War mode → {} for conn {}", war, conn_id);
            }
        }

        0xAD => {
            // Speech request — route `[commands` or broadcast to nearby.
            if let Some((speech_type, hue, font, text)) = packets::parse_speech_request(packet) {
                debug!("Speech from conn {}: {:?}", conn_id, text);

                if text.starts_with('[') {
                    handle_command(text.trim_start_matches('['), conn_id, connections);
                    return Ok(());
                }

                let (serial, name, pos) = {
                    let conn = connections.get_connection(conn_id);
                    let serial = conn.as_ref().map(|c| c.serial).unwrap_or(1);
                    let name = conn
                        .as_ref()
                        .and_then(|c| c.character_name.clone())
                        .unwrap_or_else(|| "Unknown".to_string());
                    let pos = conn
                        .as_ref()
                        .and_then(|c| c.position)
                        .unwrap_or(Position { x: 1496, y: 1628, z: 10 });
                    (serial, name, pos)
                };
                let pkt = packets::player_speech_packet(
                    serial, 0x0190, speech_type, hue, font, &name, &text,
                );
                let compressed = compress_packet(&pkt);
                connections.broadcast_to_range(conn_id, pos, 12, compressed.clone());
                connections.send_to(conn_id, compressed);
            }
        }

        0x73 => { /* ping — no response needed */ }

        _ => {
            if id != 0x00 {
                warn!("Connection {}: unhandled packet 0x{:02X}", conn_id, id);
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Outbound helpers
// ---------------------------------------------------------------------------

fn compress_packet(src: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    huffman::compress(src.to_vec(), &mut output);
    output
}

/// Enqueue the full in-game initialisation sequence for a newly-entered player.
fn send_login_init_sequence(conn_id: u64, connections: &ConnectionManager, ch: &Character) {
    use crate::status_packets::{StatBarData, StatusTypeFlag, build_status_bar_packet};

    let serial = ch.serial;
    let x = ch.position.0;
    let y = ch.position.1;
    let z = ch.position.2;
    let body = ch.body_type;
    let map_width: u16 = 6144;
    let map_height: u16 = 4096;

    // 0x1B Login Confirm
    connections.send_to(
        conn_id,
        compress_packet(&packets::login_confirm_packet(serial, body, x, y, z, 0x00, map_width, map_height)),
    );
    debug!("Queued Login Confirm (0x1B)");

    // 0x11 Status Bar (using real character stats)
    let hp = &ch.derived_stats.hit_points;
    let stam = &ch.derived_stats.stamina;
    let mana = &ch.derived_stats.mana;
    let stat_data = StatBarData {
        serial,
        name: ch.name.clone(),
        hit_points: hp.current as u16,
        max_hit_points: hp.max as u16,
        stamina: stam.current as u16,
        max_stamina: stam.max as u16,
        mana: mana.current as u16,
        max_mana: mana.max as u16,
        str_stat: ch.stats.strength as u16,
        dex_stat: ch.stats.dexterity as u16,
        int_stat: ch.stats.intelligence as u16,
        ..StatBarData::default()
    };
    connections.send_to(
        conn_id,
        compress_packet(&build_status_bar_packet(&stat_data, StatusTypeFlag::Basic)),
    );
    debug!("Queued Status Bar (0x11)");

    // 0x20 Draw Player
    let position = Position { x, y, z };
    let draw_pkt = packets::draw_player_packet(serial, body, ch.hue, 0x00, position, Direction::North, 0x01);
    connections.send_to(conn_id, compress_packet(&draw_pkt));
    debug!("Queued Draw Player (0x20)");

    // Lights
    connections.send_to(conn_id, compress_packet(&packets::global_light_level_packet(15)));
    connections.send_to(conn_id, compress_packet(&packets::personal_light_level_packet(serial, 15)));

    // 0x72 War Mode (peace)
    connections.send_to(conn_id, compress_packet(&packets::war_mode_packet(false)));

    // 0x55 Login Complete
    connections.send_to(conn_id, compress_packet(&packets::login_complete_packet()));
    debug!("Queued Login Complete (0x55)");

    info!("In-game init sequence queued for serial 0x{:08X}", serial);
}

/// Build a 0x11 status-bar packet using the live character data for `conn_id`.
fn build_status_packet_for(conn_id: u64, connections: &ConnectionManager) -> Option<Vec<u8>> {
    use crate::status_packets::{StatBarData, StatusTypeFlag, build_status_bar_packet};
    let conn = connections.get_connection(conn_id)?;
    let ch = conn.character.as_ref()?;
    let hp = &ch.derived_stats.hit_points;
    let stam = &ch.derived_stats.stamina;
    let mana = &ch.derived_stats.mana;
    let data = StatBarData {
        serial: ch.serial,
        name: ch.name.clone(),
        hit_points: hp.current as u16,
        max_hit_points: hp.max as u16,
        stamina: stam.current as u16,
        max_stamina: stam.max as u16,
        mana: mana.current as u16,
        max_mana: mana.max as u16,
        str_stat: ch.stats.strength as u16,
        dex_stat: ch.stats.dexterity as u16,
        int_stat: ch.stats.intelligence as u16,
        ..StatBarData::default()
    };
    Some(build_status_bar_packet(&data, StatusTypeFlag::Basic))
}

// ---------------------------------------------------------------------------
// Packet parsers (no I/O — pure data extraction)
// ---------------------------------------------------------------------------

fn read_u8(input: &mut &[u8]) -> Result<u8> {
    if input.is_empty() {
        return Err(ServerError::PacketParse("not enough bytes to read u8".into()));
    }
    let (b, rest) = input.split_at(1);
    *input = rest;
    Ok(b[0])
}

fn read_u16(input: &mut &[u8]) -> Result<u16> {
    if input.len() < 2 {
        return Err(ServerError::PacketParse("not enough bytes to read u16".into()));
    }
    let (b, rest) = input.split_at(2);
    *input = rest;
    Ok(u16::from_be_bytes(b.try_into()?))
}

fn read_u32(input: &mut &[u8]) -> Result<u32> {
    if input.len() < 4 {
        return Err(ServerError::PacketParse("not enough bytes to read u32".into()));
    }
    let (b, rest) = input.split_at(4);
    *input = rest;
    Ok(u32::from_be_bytes(b.try_into()?))
}

fn read_string<'a>(input: &'a mut &[u8], length: u8) -> Result<&'a str> {
    let len = length as usize;
    if input.len() < len {
        return Err(ServerError::PacketParse(format!(
            "not enough bytes to read string of length {}",
            length
        )));
    }
    let (s, rest) = input.split_at(len);
    *input = rest;
    Ok(str::from_utf8(s)?)
}

fn handle_encrypted_login_seed_packet(slice: &mut &[u8]) -> Result<(u8, u8, u8, u8)> {
    debug!("Encrypted Login Seed packet received");
    let _seed = read_u32(slice)?;
    let major = read_u32(slice)? as u8;
    let minor = read_u32(slice)? as u8;
    let revision = read_u32(slice)? as u8;
    let patch = read_u32(slice)? as u8;
    info!("Client version: {}.{}.{}.{}", major, minor, revision, patch);
    Ok((major, minor, revision, patch))
}

fn handle_account_login_request_packet(slice: &mut &[u8]) -> Result<String> {
    debug!("Account Login Request packet received");
    let username = read_string(slice, 30)?.trim_end_matches('\0').to_string();
    info!("Login request from user: {}", username);
    let _password = read_string(slice, 30)?;
    trace!("password received");
    Ok(username)
}

fn handle_server_select_packet(slice: &mut &[u8]) -> Result<u16> {
    debug!("Server Select packet received");
    let index = read_u16(slice)?;
    debug!("server_index: {}", index);
    Ok(index)
}

fn handle_post_login_packet(slice: &mut &[u8]) -> Result<String> {
    debug!("Post Login packet received");
    let _key = read_u32(slice)?;
    let username = read_string(slice, 30)?.trim_end_matches('\0').to_string();
    info!("Post-login from user: {}", username);
    let _password = read_string(slice, 30)?;
    trace!("password received");
    Ok(username)
}

fn handle_character_select_packet(slice: &mut &[u8]) -> Result<(u32, String)> {
    debug!("Character Select packet received");
    match packets::parse_character_select(slice) {
        Some((slot, name)) => {
            info!("Character select: slot={}, name={}", slot, name);
            Ok((slot, name))
        }
        None => Err(ServerError::PacketParse(
            "failed to parse character select packet".into(),
        )),
    }
}

fn handle_movement_request(slice: &mut &[u8]) -> Result<Option<MovementRequest>> {
    if slice.len() < 6 {
        return Err(ServerError::PacketParse(
            "not enough bytes for movement request".into(),
        ));
    }
    let (data, rest) = slice.split_at(6);
    *slice = rest;
    let arr: &[u8; 6] = data
        .try_into()
        .map_err(|_| ServerError::PacketParse("movement request not 6 bytes".into()))?;
    Ok(packets::parse_movement_request(arr))
}

fn apply_movement(pos: Position, dir: Direction) -> Position {
    let (dx, dy): (i32, i32) = match dir {
        Direction::North => (0, -1),
        Direction::Northeast => (1, -1),
        Direction::East => (1, 0),
        Direction::Southeast => (1, 1),
        Direction::South => (0, 1),
        Direction::Southwest => (-1, 1),
        Direction::West => (-1, 0),
        Direction::Northwest => (-1, -1),
    };
    Position {
        x: (pos.x as i32 + dx).max(0) as u16,
        y: (pos.y as i32 + dy).max(0) as u16,
        z: pos.z,
    }
}

// ---------------------------------------------------------------------------
// Command handler
// ---------------------------------------------------------------------------

/// Handle a `[command` typed in chat.  `cmd` is the text after the `[`.
fn handle_command(cmd: &str, conn_id: u64, connections: &ConnectionManager) {
    match cmd.trim().to_lowercase().as_str() {
        "res" => resurrect_player(conn_id, connections),
        _ => {
            let msg = packets::system_message_packet(
                &format!("Unknown command: [{}. Try [res", cmd.trim()),
            );
            connections.send_to(conn_id, compress_packet(&msg));
        }
    }
}

/// Restore a dead player to life with 10 % of max HP.
///
/// Sends the resurrect packet (0x2C), an updated status bar, and a system
/// message.  Does nothing if the player is already alive.
fn resurrect_player(conn_id: u64, connections: &ConnectionManager) {
    let resurrected = connections
        .with_connection_mut(conn_id, |c| {
            if let Some(ref mut ch) = c.character {
                if !ch.is_alive {
                    ch.is_alive = true;
                    let restored = (ch.derived_stats.hit_points.max / 10).max(1);
                    ch.derived_stats.hit_points.current = restored;
                    return true;
                }
            }
            false
        })
        .unwrap_or(false);

    if !resurrected {
        return;
    }

    connections.send_to(conn_id, compress_packet(&packets::resurrect_packet()));

    if let Some(status_pkt) = build_status_packet_for(conn_id, connections) {
        connections.send_to(conn_id, compress_packet(&status_pkt));
    }

    let msg = packets::system_message_packet("You have been resurrected.");
    connections.send_to(conn_id, compress_packet(&msg));
    info!("Connection {} resurrected", conn_id);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    /// Verify that the 0x11 status bar packet bytes are built correctly
    /// using the status_packets module (no async I/O needed for this check).
    #[test]
    fn status_bar_packet_builds_correctly() {
        use crate::status_packets::{StatBarData, StatusTypeFlag, build_status_bar_packet};

        let data = StatBarData {
            serial: 0x0000_0001,
            name: "Player".to_string(),
            hit_points: 100,
            max_hit_points: 100,
            stamina: 100,
            max_stamina: 100,
            mana: 100,
            max_mana: 100,
            str_stat: 100,
            dex_stat: 100,
            int_stat: 100,
            ..StatBarData::default()
        };

        let pkt = build_status_bar_packet(&data, StatusTypeFlag::Basic);

        assert_eq!(pkt[0], 0x11);
        let encoded_len = ((pkt[1] as u16) << 8) | (pkt[2] as u16);
        assert_eq!(encoded_len as usize, pkt.len());
        assert_eq!(&pkt[3..7], &[0x00, 0x00, 0x00, 0x01]);
        assert_eq!(&pkt[7..13], b"Player");
        assert!(pkt[13..37].iter().all(|&b| b == 0));
        assert_eq!(pkt[42], 0x00);
    }

    #[test]
    fn ankh_positions_are_all_in_felucca_bounds() {
        for &(x, y) in super::ANKH_POSITIONS {
            assert!(x < 7168, "Ankh x={} out of Felucca bounds", x);
            assert!(y < 4096, "Ankh y={} out of Felucca bounds", y);
        }
    }

    #[test]
    fn required_packet_length_known_ids() {
        use super::required_packet_length;
        assert_eq!(required_packet_length(0xEF, &[0xEF; 21]), Some(21));
        assert_eq!(required_packet_length(0x80, &[0x80; 62]), Some(62));
        assert_eq!(required_packet_length(0x73, &[0x73, 0x00]), Some(2));
        assert_eq!(required_packet_length(0x02, &[0x02; 7]), Some(7));
    }

    #[test]
    fn required_packet_length_variable_returns_none_when_short() {
        use super::required_packet_length;
        let short = [0xAD, 0x00]; // only 2 bytes — need 3 for length field
        assert_eq!(required_packet_length(0xAD, &short), None);
    }

    #[test]
    fn required_packet_length_variable_reads_length() {
        use super::required_packet_length;
        let buf = [0xAD, 0x00, 0x1A, 0x00, 0x00]; // length = 26
        assert_eq!(required_packet_length(0xAD, &buf), Some(26));
    }

    #[test]
    fn required_packet_length_unknown_returns_none() {
        use super::required_packet_length;
        assert_eq!(required_packet_length(0xFF, &[0xFF]), None);
    }
}
