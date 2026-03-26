use std::str;
use async_std::{net::{TcpListener, TcpStream, ToSocketAddrs}, prelude::*, task};
use log::{debug, info, trace, warn};
use crate::error::ServerError;
use crate::huffman;

mod packets;

type Result<T> = std::result::Result<T, ServerError>;

async fn connection_loop(mut stream: TcpStream) -> Result<()> {
    let mut buffer = [0; 1024];
    while let Ok(received) = stream.read(&mut buffer).await {
        if received == 0 { let addr = stream.peer_addr()?; info!("Connection closed by: {}", addr); break; }
        else { parse_packets(buffer, &mut stream).await?; buffer = [0; 1024]; }
    }
    Ok(())
}

async fn accept_loop(addr: impl ToSocketAddrs) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!("TCP listener bound, waiting for connections");
    let mut incoming = listener.incoming();
    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        let addr = stream.peer_addr()?;
        info!("Connection received from: {}", addr);
        task::spawn(connection_loop(stream));
    }
    Ok(())
}

pub fn start() -> Result<()> {
    info!("Starting TCP server on 127.0.0.1:2593");
    task::block_on(accept_loop("127.0.0.1:2593"))
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

fn handle_encrypted_login_seed_packet(buffer_slice: &mut &[u8]) -> Result<()> {
    debug!("Encrypted Login Seed packet received");
    let packet_length = 20;
    if buffer_slice.len() < packet_length { return Err(ServerError::PacketParse("not enough bytes for encrypted login seed packet".into())); }
    let (mut bytes, rest) = buffer_slice.split_at(packet_length); *buffer_slice = rest;
    let seed = read_u32(&mut bytes)?; debug!("seed: {}", seed);
    let major = read_u32(&mut bytes)?; let minor = read_u32(&mut bytes)?;
    let revision = read_u32(&mut bytes)?; let patch = read_u32(&mut bytes)?;
    info!("Client version: {}.{}.{}.{}", major, minor, revision, patch);
    Ok(())
}

fn handle_account_login_request_packet(buffer_slice: &mut &[u8]) -> Result<()> {
    debug!("Account Login Request packet received");
    let packet_length = 61;
    if buffer_slice.len() < packet_length { return Err(ServerError::PacketParse("not enough bytes for account login request packet".into())); }
    let (mut bytes, rest) = buffer_slice.split_at(packet_length); *buffer_slice = rest;
    let username = read_string(&mut bytes, 30)?; info!("Login request from user: {}", username);
    let password = read_string(&mut bytes, 30)?; trace!("password: {}", password);
    Ok(())
}

fn handle_server_select_packet(buffer_slice: &mut &[u8]) -> Result<()> {
    debug!("Server Select packet received");
    let packet_length = 2;
    if buffer_slice.len() < packet_length { return Err(ServerError::PacketParse("not enough bytes for server select packet".into())); }
    let (mut bytes, rest) = buffer_slice.split_at(packet_length); *buffer_slice = rest;
    let server_index = read_u16(&mut bytes)?; debug!("server_index: {}", server_index);
    Ok(())
}

fn handle_post_login_packet(buffer_slice: &mut &[u8]) -> Result<()> {
    debug!("Post Login packet received");
    let packet_length = 64;
    if buffer_slice.len() < packet_length { return Err(ServerError::PacketParse("not enough bytes for post login packet".into())); }
    let (mut bytes, rest) = buffer_slice.split_at(packet_length); *buffer_slice = rest;
    let encryption_key = read_u32(&mut bytes)?; debug!("encryption_key: {}", encryption_key);
    let username = read_string(&mut bytes, 30)?; info!("Post-login from user: {}", username);
    let password = read_string(&mut bytes, 30)?; trace!("password: {}", password);
    Ok(())
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

async fn parse_packets(buffer: [u8; 1024], mut stream: &mut TcpStream) -> Result<()> {
    let mut buffer_slice = &buffer[..];
    debug!("Parsing packet");
    while buffer_slice.len() > 0 {
        let packet_id = read_u8(&mut buffer_slice)?;
        match packet_id {
            0xEF => { handle_encrypted_login_seed_packet(&mut buffer_slice)?; }
            0x80 => { handle_account_login_request_packet(&mut buffer_slice)?; send_server_list_packet(&mut stream).await?; }
            0xA0 => { handle_server_select_packet(&mut buffer_slice)?; send_server_redirect_packet(&mut stream).await?; }
            0x91 => { handle_post_login_packet(&mut buffer_slice)?; send_features_packet(&mut stream).await?; send_character_list_packet(&mut stream).await?; }
            0x73 => continue,
            _ => { if packet_id != 0x00 { warn!("Unknown packet ID: 0x{:02X}", packet_id); } continue; }
        }
    }
    debug!("Finished parsing packet");
    Ok(())
}
