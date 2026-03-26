use std::str;
use async_std::{net::{TcpListener, TcpStream, ToSocketAddrs}, prelude::*, task};
use log::{debug, info, trace, warn};
use crate::error::ServerError;
use crate::huffman;
use crate::connections::ConnectionManager;
use crate::connection::ConnectionState;
use crate::movement::{Direction, Position};

mod packets;

type Result<T> = std::result::Result<T, ServerError>;

async fn connection_loop(mut stream: TcpStream, connections: ConnectionManager) -> Result<()> {
    let addr = stream.peer_addr()?;
    let conn_id = connections.add_connection(addr);
    info!("Registered connection {} from: {}", conn_id, addr);

    let mut buffer = [0; 1024];
    while let Ok(received) = stream.read(&mut buffer).await {
        if received == 0 {
            info!("Connection {} closed by: {}", conn_id, addr);
            connections.with_connection_mut(conn_id, |c| {
                let _ = c.transition_to(ConnectionState::Disconnected);
            });
            connections.remove_connection(conn_id);
            break;
        }
        if let Err(e) = parse_packets(buffer, &mut stream, conn_id, &connections).await {
            warn!("Connection {} packet error: {}", conn_id, e);
        }
        buffer = [0; 1024];
    }
    Ok(())
}

async fn accept_loop(addr: impl ToSocketAddrs, connections: ConnectionManager) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!("TCP listener bound, waiting for connections");
    let mut incoming = listener.incoming();
    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        let addr = stream.peer_addr()?;
        info!("Connection received from: {}", addr);
        let conns = connections.clone();
        task::spawn(connection_loop(stream, conns));
    }
    Ok(())
}

pub fn start(connections: ConnectionManager) -> Result<()> {
    info!("Starting TCP server on 127.0.0.1:2593");
    task::block_on(accept_loop("127.0.0.1:2593", connections))
}

fn read_u8(input: &mut &[u8]) -> Result<u8> {
    if input.is_empty() { return Err(ServerError::PacketParse("not enough bytes to read u8".into())); }
    let (int_bytes, rest) = input.split_at(1); *input = rest; Ok(int_bytes[0])
}

fn read_u16(input: &mut &[u8]) -> Result<u16> {
    if input.len() < 2 { return Err(ServerError::PacketParse("not enough bytes to read u16".into())); }
    let (int_bytes, rest) = input.split_at(2); *input = rest; Ok(u16::from_be_bytes(int_bytes.try_into()?))
}

fn read_u32(input: &mut &[u8]) -> Result<u32> {
    if input.len() < 4 { return Err(ServerError::PacketParse("not enough bytes to read u32".into())); }
    let (int_bytes, rest) = input.split_at(4); *input = rest; Ok(u32::from_be_bytes(int_bytes.try_into()?))
}

fn read_string<'a>(input: &'a mut &[u8], length: u8) -> Result<&'a str> {
    let len = length as usize;
    if input.len() < len { return Err(ServerError::PacketParse(format!("not enough bytes to read string of length {}", length))); }
    let (string_bytes, rest) = input.split_at(len); *input = rest; Ok(str::from_utf8(string_bytes)?)
}

fn handle_encrypted_login_seed_packet(buffer_slice: &mut &[u8]) -> Result<(u8, u8, u8, u8)> {
    debug!("Encrypted Login Seed packet received");
    let packet_length = 20;
    if buffer_slice.len() < packet_length { return Err(ServerError::PacketParse("not enough bytes for encrypted login seed packet".into())); }
    let (mut bytes, rest) = buffer_slice.split_at(packet_length); *buffer_slice = rest;
    let seed = read_u32(&mut bytes)?; debug!("seed: {}", seed);
    let major = read_u32(&mut bytes)?;
    let minor = read_u32(&mut bytes)?;
    let revision = read_u32(&mut bytes)?;
    let patch = read_u32(&mut bytes)?;
    info!("Client version: {}.{}.{}.{}", major, minor, revision, patch);
    Ok((major as u8, minor as u8, revision as u8, patch as u8))
}

fn handle_account_login_request_packet(buffer_slice: &mut &[u8]) -> Result<String> {
    debug!("Account Login Request packet received");
    let packet_length = 61;
    if buffer_slice.len() < packet_length { return Err(ServerError::PacketParse("not enough bytes for account login request packet".into())); }
    let (mut bytes, rest) = buffer_slice.split_at(packet_length); *buffer_slice = rest;
    let username = read_string(&mut bytes, 30)?.trim_end_matches('\0').to_string();
    info!("Login request from user: {}", username);
    let _password = read_string(&mut bytes, 30)?;
    trace!("password received");
    Ok(username)
}

fn handle_server_select_packet(buffer_slice: &mut &[u8]) -> Result<u16> {
    debug!("Server Select packet received");
    let packet_length = 2;
    if buffer_slice.len() < packet_length { return Err(ServerError::PacketParse("not enough bytes for server select packet".into())); }
    let (mut bytes, rest) = buffer_slice.split_at(packet_length); *buffer_slice = rest;
    let server_index = read_u16(&mut bytes)?; debug!("server_index: {}", server_index);
    Ok(server_index)
}

fn handle_post_login_packet(buffer_slice: &mut &[u8]) -> Result<String> {
    debug!("Post Login packet received");
    let packet_length = 64;
    if buffer_slice.len() < packet_length { return Err(ServerError::PacketParse("not enough bytes for post login packet".into())); }
    let (mut bytes, rest) = buffer_slice.split_at(packet_length); *buffer_slice = rest;
    let encryption_key = read_u32(&mut bytes)?; debug!("encryption_key: {}", encryption_key);
    let username = read_string(&mut bytes, 30)?.trim_end_matches('\0').to_string();
    info!("Post-login from user: {}", username);
    let _password = read_string(&mut bytes, 30)?;
    trace!("password received");
    Ok(username)
}

fn handle_character_select_packet(buffer_slice: &mut &[u8]) -> Result<(u32, String)> {
    debug!("Character Select packet received");
    // 0x5D is 73 bytes total; the packet ID byte was already consumed
    let packet_length = 72;
    if buffer_slice.len() < packet_length { return Err(ServerError::PacketParse("not enough bytes for character select packet".into())); }
    let (data, rest) = buffer_slice.split_at(packet_length); *buffer_slice = rest;
    match packets::parse_character_select(data) {
        Some((slot, name)) => { info!("Character select: slot={}, name={}", slot, name); Ok((slot, name)) }
        None => Err(ServerError::PacketParse("failed to parse character select packet".into())),
    }
}

async fn send_server_list_packet(stream: &mut TcpStream) -> Result<()> {
    let buffer = packets::server_list_packet();
    stream.write_all(&buffer).await?; stream.flush().await?;
    debug!("Sent Server List packet"); trace!("Server List packet bytes: {:X?}", buffer);
    Ok(())
}

async fn send_server_redirect_packet(stream: &mut TcpStream) -> Result<()> {
    let buffer = packets::server_redirect_packet();
    stream.write_all(&buffer).await?; stream.flush().await?;
    debug!("Sent Server Redirect packet"); trace!("Server Redirect packet bytes: {:X?}", buffer);
    Ok(())
}

async fn send_features_packet(stream: &mut TcpStream) -> Result<()> {
    let src = packets::features_packet();
    let mut output = Vec::new();
    debug!("Compressing Features packet"); trace!("Features packet uncompressed bytes: {:X?}", src);
    huffman::compress(src, &mut output);
    stream.write_all(&output).await?; stream.flush().await?;
    debug!("Sent compressed Features packet"); trace!("Features packet compressed bytes: {:X?}", output);
    Ok(())
}

async fn send_character_list_packet(stream: &mut TcpStream) -> Result<()> {
    let src = packets::character_list_packet();
    let mut output = Vec::new();
    debug!("Compressing Character List packet"); trace!("Character List packet uncompressed bytes: {:02X?}", src);
    huffman::compress(src, &mut output);
    stream.write_all(&output).await?; stream.flush().await?;
    debug!("Sent compressed Character List packet"); trace!("Character List packet compressed bytes: {:X?}", output);
    Ok(())
}

fn compress_packet(src: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    huffman::compress(src.to_vec(), &mut output);
    output
}

async fn send_login_init_sequence(stream: &mut TcpStream, serial: u32) -> Result<()> {
    // Starting position: Britain (Felucca)
    let x: u16 = 1496;
    let y: u16 = 1628;
    let z: i8 = 10;
    let body: u16 = 0x0190; // Male human
    let map_width: u16 = 6144;
    let map_height: u16 = 4096;

    // 0x1B Login Confirm
    stream.write_all(&compress_packet(&packets::login_confirm_packet(serial, body, x, y, z, 0x00, map_width, map_height))).await?;
    stream.flush().await?;
    debug!("Sent Login Confirm (0x1B)");

    // 0x20 Draw Player
    let position = Position { x, y, z };
    let draw_pkt = packets::draw_player_packet(serial, body, 0x0000, 0x00, position, Direction::North, 0x01);
    stream.write_all(&compress_packet(&draw_pkt)).await?;
    stream.flush().await?;
    debug!("Sent Draw Player (0x20)");

    // 0x4F Global Light Level
    stream.write_all(&compress_packet(&packets::global_light_level_packet(15))).await?;
    stream.flush().await?;
    debug!("Sent Global Light Level (0x4F)");

    // 0x4E Personal Light Level
    stream.write_all(&compress_packet(&packets::personal_light_level_packet(serial, 15))).await?;
    stream.flush().await?;
    debug!("Sent Personal Light Level (0x4E)");

    // 0x72 War Mode
    stream.write_all(&compress_packet(&packets::war_mode_packet(false))).await?;
    stream.flush().await?;
    debug!("Sent War Mode (0x72)");

    // 0x55 Login Complete
    stream.write_all(&compress_packet(&packets::login_complete_packet())).await?;
    stream.flush().await?;
    debug!("Sent Login Complete (0x55)");

    info!("In-game init sequence complete for serial 0x{:08X}", serial);
    Ok(())
}

async fn parse_packets(buffer: [u8; 1024], mut stream: &mut TcpStream, conn_id: u64, connections: &ConnectionManager) -> Result<()> {
    let mut buffer_slice = &buffer[..];
    debug!("Parsing packet");
    while buffer_slice.len() > 0 {
        let packet_id = read_u8(&mut buffer_slice)?;
        match packet_id {
            0xEF => {
                let (major, minor, revision, patch) = handle_encrypted_login_seed_packet(&mut buffer_slice)?;
                connections.with_connection_mut(conn_id, |c| {
                    c.set_client_version(major, minor, revision, patch);
                    let _ = c.transition_to(ConnectionState::LoginSeed);
                });
            }
            0x80 => {
                let username = handle_account_login_request_packet(&mut buffer_slice)?;
                connections.with_connection_mut(conn_id, |c| {
                    c.set_account_name(username.clone());
                    let _ = c.transition_to(ConnectionState::Authenticating);
                    let _ = c.transition_to(ConnectionState::ServerSelect);
                });
                send_server_list_packet(&mut stream).await?;
            }
            0xA0 => {
                handle_server_select_packet(&mut buffer_slice)?;
                connections.with_connection_mut(conn_id, |c| {
                    let _ = c.transition_to(ConnectionState::GameLogin);
                });
                send_server_redirect_packet(&mut stream).await?;
            }
            0x91 => {
                let username = handle_post_login_packet(&mut buffer_slice)?;
                connections.with_connection_mut(conn_id, |c| {
                    // After a redirect the client reconnects; the new connection
                    // starts at Connecting and must reach GameLogin via 0x91.
                    if c.state == ConnectionState::Connecting {
                        let _ = c.transition_to(ConnectionState::LoginSeed);
                        let _ = c.transition_to(ConnectionState::Authenticating);
                        let _ = c.transition_to(ConnectionState::ServerSelect);
                        let _ = c.transition_to(ConnectionState::GameLogin);
                    }
                    c.set_account_name(username.clone());
                });
                send_features_packet(&mut stream).await?;
                send_character_list_packet(&mut stream).await?;
            }
            0x5D => {
                let (slot, char_name) = handle_character_select_packet(&mut buffer_slice)?;
                // Assign a placeholder serial from the slot number
                let serial = 0x00000001u32 + slot;
                connections.with_connection_mut(conn_id, |c| {
                    c.set_character_name(char_name.clone());
                    let _ = c.transition_to(ConnectionState::InGame);
                });
                info!("Player entering game: {} (serial 0x{:08X})", char_name, serial);
                send_login_init_sequence(&mut stream, serial).await?;
            }
            0x73 => continue,
            _ => { if packet_id != 0x00 { warn!("Unknown packet ID: 0x{:02X}", packet_id); } continue; }
        }
    }
    debug!("Finished parsing packet");
    Ok(())
}
