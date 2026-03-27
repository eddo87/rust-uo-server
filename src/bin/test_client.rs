//! UO Protocol test client.
//!
//! Connects to 127.0.0.1:2593 (server must already be running) and exercises
//! the full login flow end-to-end, reporting PASS/FAIL for each step.
//!
//! Run:
//!   cargo run --bin test_client
//!
//! Exits 0 if all tests pass, non-zero otherwise.

use async_std::{net::TcpStream, prelude::*, task};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Low-level client
// ---------------------------------------------------------------------------

struct UoClient {
    stream: TcpStream,
    acc: Vec<u8>,
}

impl UoClient {
    async fn connect(addr: &str) -> Result<Self, String> {
        let stream = async_std::io::timeout(
            Duration::from_secs(3),
            TcpStream::connect(addr),
        )
        .await
        .map_err(|e| format!("connect error to {}: {}", addr, e))?;
        Ok(Self { stream, acc: Vec::new() })
    }

    async fn send(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.stream
            .write_all(bytes)
            .await
            .map_err(|e| format!("send error: {}", e))?;
        self.stream.flush().await.map_err(|e| format!("flush error: {}", e))
    }

    /// Fill internal accumulator with at least one read.
    async fn fill(&mut self) -> Result<(), String> {
        let mut buf = [0u8; 4096];
        let n = async_std::io::timeout(Duration::from_secs(5), self.stream.read(&mut buf))
            .await
            .map_err(|e| format!("read error: {}", e))?;
        if n == 0 {
            return Err("connection closed by server".into());
        }
        self.acc.extend_from_slice(&buf[..n]);
        Ok(())
    }

    /// Read one complete uncompressed packet (login-server phase only).
    /// Understands fixed-length and variable-length (length in bytes 1-2) packets.
    async fn recv_login_packet(&mut self) -> Result<Vec<u8>, String> {
        while self.acc.is_empty() {
            self.fill().await?;
        }
        let id = self.acc[0];
        let len: usize = match id {
            // variable-length: total length in big-endian bytes [1..3]
            0xA8 | 0xB0 | 0xBD | 0xBF => {
                while self.acc.len() < 3 {
                    self.fill().await?;
                }
                u16::from_be_bytes([self.acc[1], self.acc[2]]) as usize
            }
            0x82 => 2,
            0x8C => 11,
            _ => return Err(format!("unexpected server packet 0x{:02X} during login phase", id)),
        };
        if len == 0 {
            return Err(format!("packet 0x{:02X} reported length 0", id));
        }
        while self.acc.len() < len {
            self.fill().await?;
        }
        let pkt = self.acc[..len].to_vec();
        self.acc.drain(..len);
        Ok(pkt)
    }

    /// Receive raw bytes from the game-server phase (compressed).
    /// Just verifies that SOMETHING arrives within the timeout.
    async fn recv_game_bytes(&mut self) -> Result<Vec<u8>, String> {
        if self.acc.is_empty() {
            self.fill().await?;
        }
        Ok(std::mem::take(&mut self.acc))
    }
}

// ---------------------------------------------------------------------------
// Packet builders (client → server)
// ---------------------------------------------------------------------------

/// 0xEF — EncryptedLoginSeed
fn pkt_ef(version: (u8, u8, u8, u8)) -> Vec<u8> {
    let mut v = vec![0xEF];
    v.extend_from_slice(&0x12345678u32.to_be_bytes()); // seed
    v.extend_from_slice(&(version.0 as u32).to_be_bytes());
    v.extend_from_slice(&(version.1 as u32).to_be_bytes());
    v.extend_from_slice(&(version.2 as u32).to_be_bytes());
    v.extend_from_slice(&(version.3 as u32).to_be_bytes());
    v // 21 bytes
}

/// 0x80 — AccountLoginRequest
fn pkt_80(username: &str, password: &str) -> Vec<u8> {
    let mut v = vec![0x80u8];
    let mut u = [0u8; 30];
    let ub = username.as_bytes();
    u[..ub.len().min(29)].copy_from_slice(&ub[..ub.len().min(29)]);
    v.extend_from_slice(&u);
    let mut p = [0u8; 30];
    let pb = password.as_bytes();
    p[..pb.len().min(29)].copy_from_slice(&pb[..pb.len().min(29)]);
    v.extend_from_slice(&p);
    v.push(0x00);
    v // 62 bytes
}

/// 0xA0 — ServerSelect
fn pkt_a0(index: u16) -> Vec<u8> {
    let mut v = vec![0xA0];
    v.extend_from_slice(&index.to_be_bytes());
    v // 3 bytes
}

/// 0x91 — GameServerLogin (sent on the game-server connection)
fn pkt_91(auth_key: &[u8; 4], username: &str, password: &str) -> Vec<u8> {
    let mut v = vec![0x91u8];
    v.extend_from_slice(auth_key);
    let mut u = [0u8; 30];
    let ub = username.as_bytes();
    u[..ub.len().min(29)].copy_from_slice(&ub[..ub.len().min(29)]);
    v.extend_from_slice(&u);
    let mut p = [0u8; 30];
    let pb = password.as_bytes();
    p[..pb.len().min(29)].copy_from_slice(&pb[..pb.len().min(29)]);
    v.extend_from_slice(&p);
    v // 65 bytes
}

/// 0xF8 — CreateCharacter (7.0.16+ clients), 106 bytes.
/// Name is placed at bytes [9..39] (after ID + 8 pattern bytes).
fn pkt_f8(name: &str) -> Vec<u8> {
    let mut v = vec![0u8; 106];
    v[0] = 0xF8;
    // Pattern bytes [1..9]: same as older 0x00 create character
    v[1] = 0x00; v[2] = 0x00; v[3] = 0x00; v[4] = 0x00;
    v[5] = 0xFF; v[6] = 0xFF; v[7] = 0xFF; v[8] = 0xFF;
    // Name at [9..39] (30 bytes, null-padded)
    let nb = name.as_bytes();
    let n = nb.len().min(29);
    v[9..9 + n].copy_from_slice(&nb[..n]);
    // Profession = 0, stats = 25/25/25, skills = 0 (defaults fine for testing)
    v
}

// ---------------------------------------------------------------------------
// Test runner
// ---------------------------------------------------------------------------

struct Results {
    passed: usize,
    failed: usize,
}

impl Results {
    fn new() -> Self { Self { passed: 0, failed: 0 } }

    fn pass(&mut self, name: &str) {
        println!("  PASS  {}", name);
        self.passed += 1;
    }

    fn fail(&mut self, name: &str, reason: &str) {
        println!("  FAIL  {}  — {}", name, reason);
        self.failed += 1;
    }

    fn check(&mut self, name: &str, result: Result<(), String>) {
        match result {
            Ok(()) => self.pass(name),
            Err(e)  => self.fail(name, &e),
        }
    }
}

// ---------------------------------------------------------------------------
// Test scenarios
// ---------------------------------------------------------------------------

/// Full login-server phase: seed → login → server list → redirect.
async fn scenario_login_server(r: &mut Results) -> Option<[u8; 4]> {
    println!("\n[Login Server]");

    // 1. Connect
    let mut client = match UoClient::connect("127.0.0.1:2593").await {
        Ok(c) => { r.pass("connect"); c }
        Err(e) => { r.fail("connect", &e); return None; }
    };

    // 2. Version too old → expect 0x82 deny
    r.check("version gate (client 3.x rejected)", async {
        let mut c = UoClient::connect("127.0.0.1:2593").await?;
        c.send(&pkt_ef((3, 0, 0, 0))).await?;
        c.send(&pkt_80("test", "test")).await?;
        let pkt = c.recv_login_packet().await?;
        if pkt[0] != 0x82 {
            return Err(format!("expected 0x82 deny, got 0x{:02X}", pkt[0]));
        }
        Ok(())
    }.await);

    // 3. Send valid seed
    r.check("send login seed (7.0.108.0)", client.send(&pkt_ef((7, 0, 108, 0))).await.map_err(|e| e));

    // 4. Send account login
    r.check("send account login", client.send(&pkt_80("testuser", "testpass")).await.map_err(|e| e));

    // 5. Expect 0xA8 server list
    let shard_name = match async {
        let pkt = client.recv_login_packet().await?;
        if pkt[0] != 0xA8 { return Err(format!("expected 0xA8, got 0x{:02X}", pkt[0])); }
        // Length in [1..3]
        let len = u16::from_be_bytes([pkt[1], pkt[2]]) as usize;
        if pkt.len() != len { return Err(format!("0xA8 length mismatch: header={} actual={}", len, pkt.len())); }
        // Server count in [4..6]
        let count = u16::from_be_bytes([pkt[4], pkt[5]]);
        if count == 0 { return Err("server list is empty".into()); }
        // Server name starts at byte 8 (32-byte field, null-terminated)
        let name_bytes = &pkt[8..40];
        let end = name_bytes.iter().position(|&b| b == 0).unwrap_or(32);
        Ok(String::from_utf8_lossy(&name_bytes[..end]).to_string())
    }.await {
        Ok(name) => { r.pass(&format!("received server list (shard: \"{}\")", name)); name }
        Err(e)   => { r.fail("received server list", &e); return None; }
    };
    let _ = shard_name;

    // 6. Select server 0
    r.check("send server select", client.send(&pkt_a0(0)).await.map_err(|e| e));

    // 7. Expect 0x8C redirect
    let auth_key = match async {
        let pkt = client.recv_login_packet().await?;
        if pkt[0] != 0x8C { return Err(format!("expected 0x8C redirect, got 0x{:02X}", pkt[0])); }
        if pkt.len() != 11 { return Err(format!("0x8C wrong length: {}", pkt.len())); }
        let ip = format!("{}.{}.{}.{}", pkt[1], pkt[2], pkt[3], pkt[4]);
        let port = u16::from_be_bytes([pkt[5], pkt[6]]);
        let key: [u8; 4] = [pkt[7], pkt[8], pkt[9], pkt[10]];
        Ok((ip, port, key))
    }.await {
        Ok((ip, port, key)) => {
            r.pass(&format!("received redirect ({}:{}, key={:02X}{:02X}{:02X}{:02X})", ip, port, key[0], key[1], key[2], key[3]));
            key
        }
        Err(e) => { r.fail("received redirect", &e); return None; }
    };

    Some(auth_key)
}

/// Game-server phase: reconnect with auth seed → game login → char list.
async fn scenario_game_server(auth_key: [u8; 4], r: &mut Results) -> bool {
    println!("\n[Game Server]");

    let mut client = match UoClient::connect("127.0.0.1:2593").await {
        Ok(c) => { r.pass("reconnect to game server"); c }
        Err(e) => { r.fail("reconnect to game server", &e); return false; }
    };

    // Send auth key as raw 4-byte seed (server consumes as null packets or recognises)
    r.check("send auth seed", client.send(&auth_key).await.map_err(|e| e));

    // Send 0x91 game login
    r.check("send game login (0x91)", client.send(&pkt_91(&auth_key, "testuser", "testpass")).await.map_err(|e| e));

    // Expect features (0xB9) + char list (0xA9) — both compressed; just verify bytes arrive
    r.check("receive game server response (features + char list)", async {
        let bytes = client.recv_game_bytes().await?;
        if bytes.is_empty() {
            return Err("no data received after 0x91".into());
        }
        Ok(())
    }.await);

    // 5. Send character creation
    r.check("send character create (0xF8)", client.send(&pkt_f8("TestHero")).await.map_err(|e| e));

    // Expect init sequence — compressed; just verify bytes arrive
    r.check("receive login init sequence (0x1B/0x11/0x20/0x55)", async {
        // Drain any already-buffered bytes from previous read first
        async_std::task::sleep(Duration::from_millis(500)).await;
        let bytes = client.recv_game_bytes().await?;
        if bytes.is_empty() {
            return Err("no data received after 0xF8".into());
        }
        Ok(())
    }.await);

    true
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

async fn run() -> Results {
    let mut r = Results::new();

    println!("UO Protocol Test Client");
    println!("Connecting to 127.0.0.1:2593 — server must be running.\n");

    let auth_key = scenario_login_server(&mut r).await;

    if let Some(key) = auth_key {
        scenario_game_server(key, &mut r).await;
    } else {
        println!("\n[Game Server] — skipped (login server phase failed)");
    }

    r
}

fn main() {
    task::block_on(async {
        let r = run().await;
        println!("\n=== {} passed, {} failed ===", r.passed, r.failed);
        std::process::exit(if r.failed == 0 { 0 } else { 1 });
    });
}
