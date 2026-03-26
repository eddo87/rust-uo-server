use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use async_std::channel::Sender;
use crate::connection::Connection;

/// Thread-safe manager for all active client connections.
///
/// Internally backed by `Arc<Mutex<HashMap<u64, Connection>>>` so it can be
/// cheaply cloned and shared across async tasks.
#[derive(Debug, Clone)]
pub struct ConnectionManager {
    connections: Arc<Mutex<HashMap<u64, Connection>>>,
    next_id: Arc<Mutex<u64>>,
    /// Per-connection outbound packet senders.
    outbound: Arc<Mutex<HashMap<u64, Sender<Vec<u8>>>>>,
}

impl ConnectionManager {
    /// Create a new, empty connection manager.
    pub fn new() -> ConnectionManager {
        ConnectionManager {
            connections: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
            outbound: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a new connection from `address`.
    ///
    /// Returns the assigned connection id.
    pub fn add_connection(&self, address: SocketAddr) -> u64 {
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;

        let connection = Connection::new(id, address);
        self.connections.lock().unwrap().insert(id, connection);
        id
    }

    /// Remove a connection by its id.
    ///
    /// Returns the removed `Connection` if it existed.
    pub fn remove_connection(&self, id: u64) -> Option<Connection> {
        self.outbound.lock().unwrap().remove(&id);
        self.connections.lock().unwrap().remove(&id)
    }

    /// Register the outbound packet sender for a connection.
    /// Called by tcp.rs immediately after add_connection.
    pub fn register_sender(&self, conn_id: u64, sender: Sender<Vec<u8>>) {
        self.outbound.lock().unwrap().insert(conn_id, sender);
    }

    /// Send a packet to a specific connection. Returns true on success.
    pub fn send_to(&self, conn_id: u64, packet: Vec<u8>) -> bool {
        let sender = self.outbound.lock().unwrap().get(&conn_id).cloned();
        if let Some(tx) = sender {
            tx.try_send(packet).is_ok()
        } else {
            false
        }
    }

    /// Broadcast a packet to all InGame connections within `radius` tiles of
    /// `pos` (Chebyshev distance), excluding `exclude_id`.
    pub fn broadcast_to_range(
        &self,
        exclude_id: u64,
        pos: crate::movement::Position,
        radius: u16,
        packet: Vec<u8>,
    ) {
        let targets: Vec<u64> = {
            let map = self.connections.lock().unwrap();
            map.values()
                .filter(|c| c.id != exclude_id)
                .filter(|c| c.state == crate::connection::ConnectionState::InGame)
                .filter(|c| {
                    if let Some(cpos) = c.position {
                        let dx = (cpos.x as i32 - pos.x as i32).unsigned_abs() as u16;
                        let dy = (cpos.y as i32 - pos.y as i32).unsigned_abs() as u16;
                        dx <= radius && dy <= radius
                    } else {
                        false
                    }
                })
                .map(|c| c.id)
                .collect()
        };
        for id in targets {
            self.send_to(id, packet.clone());
        }
    }

    /// Retrieve a clone of the connection with the given id.
    pub fn get_connection(&self, id: u64) -> Option<Connection> {
        self.connections.lock().unwrap().get(&id).cloned()
    }

    /// Apply a mutating function to the connection with the given id while
    /// holding the lock. Returns `None` if the id does not exist, otherwise
    /// returns the result of the closure.
    pub fn with_connection_mut<F, R>(&self, id: u64, f: F) -> Option<R>
    where
        F: FnOnce(&mut Connection) -> R,
    {
        let mut map = self.connections.lock().unwrap();
        map.get_mut(&id).map(f)
    }

    /// Return clones of all active connections.
    pub fn get_all_connections(&self) -> Vec<Connection> {
        self.connections.lock().unwrap().values().cloned().collect()
    }

    /// Return the number of active connections.
    pub fn count(&self) -> usize {
        self.connections.lock().unwrap().len()
    }

    /// Find the connection id for the connection whose serial matches.
    pub fn find_by_serial(&self, serial: u32) -> Option<u64> {
        self.connections
            .lock()
            .unwrap()
            .values()
            .find(|c| c.serial == serial)
            .map(|c| c.id)
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::ConnectionState;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn addr(port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port)
    }

    #[test]
    fn add_and_count() {
        let mgr = ConnectionManager::new();
        assert_eq!(mgr.count(), 0);

        mgr.add_connection(addr(1000));
        assert_eq!(mgr.count(), 1);

        mgr.add_connection(addr(1001));
        assert_eq!(mgr.count(), 2);
    }

    #[test]
    fn ids_are_unique_and_sequential() {
        let mgr = ConnectionManager::new();
        let id1 = mgr.add_connection(addr(1000));
        let id2 = mgr.add_connection(addr(1001));
        let id3 = mgr.add_connection(addr(1002));
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }

    #[test]
    fn get_connection_returns_correct_data() {
        let mgr = ConnectionManager::new();
        let id = mgr.add_connection(addr(5555));

        let conn = mgr.get_connection(id).unwrap();
        assert_eq!(conn.id, id);
        assert_eq!(conn.address, addr(5555));
        assert_eq!(conn.state, ConnectionState::Connecting);
    }

    #[test]
    fn get_connection_returns_none_for_unknown_id() {
        let mgr = ConnectionManager::new();
        assert!(mgr.get_connection(999).is_none());
    }

    #[test]
    fn remove_connection_removes_and_returns() {
        let mgr = ConnectionManager::new();
        let id = mgr.add_connection(addr(1000));
        assert_eq!(mgr.count(), 1);

        let removed = mgr.remove_connection(id);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, id);
        assert_eq!(mgr.count(), 0);
    }

    #[test]
    fn remove_connection_returns_none_for_unknown_id() {
        let mgr = ConnectionManager::new();
        assert!(mgr.remove_connection(42).is_none());
    }

    #[test]
    fn get_all_connections() {
        let mgr = ConnectionManager::new();
        mgr.add_connection(addr(1000));
        mgr.add_connection(addr(1001));
        mgr.add_connection(addr(1002));

        let all = mgr.get_all_connections();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn with_connection_mut_transitions_state() {
        let mgr = ConnectionManager::new();
        let id = mgr.add_connection(addr(1000));

        let result = mgr.with_connection_mut(id, |conn| {
            conn.transition_to(ConnectionState::LoginSeed)
        });
        assert!(result.is_some());
        assert!(result.unwrap().is_ok());

        let conn = mgr.get_connection(id).unwrap();
        assert_eq!(conn.state, ConnectionState::LoginSeed);
    }

    #[test]
    fn with_connection_mut_returns_none_for_unknown_id() {
        let mgr = ConnectionManager::new();
        let result = mgr.with_connection_mut(999, |conn| {
            conn.transition_to(ConnectionState::LoginSeed)
        });
        assert!(result.is_none());
    }

    #[test]
    fn full_flow_through_manager() {
        let mgr = ConnectionManager::new();
        let id = mgr.add_connection(addr(2000));

        // Walk through the entire login flow via the manager.
        let states = [
            ConnectionState::LoginSeed,
            ConnectionState::Authenticating,
            ConnectionState::ServerSelect,
            ConnectionState::GameLogin,
            ConnectionState::InGame,
        ];

        for state in &states {
            let res = mgr
                .with_connection_mut(id, |conn| conn.transition_to(*state))
                .unwrap();
            assert!(res.is_ok(), "Transition to {} should succeed", state);
        }

        let conn = mgr.get_connection(id).unwrap();
        assert!(conn.is_in_game());
    }

    #[test]
    fn manager_is_clone_safe() {
        let mgr = ConnectionManager::new();
        let mgr2 = mgr.clone();

        let id = mgr.add_connection(addr(3000));
        // The clone should see the same connection.
        assert_eq!(mgr2.count(), 1);
        assert!(mgr2.get_connection(id).is_some());
    }

    #[test]
    fn default_creates_empty_manager() {
        let mgr = ConnectionManager::default();
        assert_eq!(mgr.count(), 0);
    }

    #[test]
    fn send_to_returns_false_for_unknown_id() {
        let mgr = ConnectionManager::new();
        assert!(!mgr.send_to(999, vec![0x00]));
    }

    #[test]
    fn register_sender_and_send_to() {
        let mgr = ConnectionManager::new();
        let id = mgr.add_connection(addr(4000));
        let (tx, rx) = async_std::channel::bounded::<Vec<u8>>(4);
        mgr.register_sender(id, tx);
        assert!(mgr.send_to(id, vec![0x78, 0x00]));
        let pkt = rx.try_recv().expect("should have received packet");
        assert_eq!(pkt, vec![0x78, 0x00]);
    }

    #[test]
    fn find_by_serial_returns_correct_id() {
        let mgr = ConnectionManager::new();
        let id = mgr.add_connection(addr(6000));
        // serial is assigned as conn_id + 1 in Connection::new
        let conn = mgr.get_connection(id).unwrap();
        let serial = conn.serial;
        assert_eq!(mgr.find_by_serial(serial), Some(id));
        assert_eq!(mgr.find_by_serial(serial + 1000), None);
    }

    #[test]
    fn broadcast_to_range_reaches_nearby_skips_far() {
        use crate::movement::Position;

        let mgr = ConnectionManager::new();

        // Broadcaster at (100, 100)
        let id_a = mgr.add_connection(addr(5001));
        mgr.with_connection_mut(id_a, |c| {
            for s in &[ConnectionState::LoginSeed, ConnectionState::Authenticating,
                       ConnectionState::ServerSelect, ConnectionState::GameLogin,
                       ConnectionState::InGame] {
                let _ = c.transition_to(*s);
            }
            c.set_position(Position { x: 100, y: 100, z: 0 });
        });

        // Player B at (105, 100) — within radius 18
        let id_b = mgr.add_connection(addr(5002));
        mgr.with_connection_mut(id_b, |c| {
            for s in &[ConnectionState::LoginSeed, ConnectionState::Authenticating,
                       ConnectionState::ServerSelect, ConnectionState::GameLogin,
                       ConnectionState::InGame] {
                let _ = c.transition_to(*s);
            }
            c.set_position(Position { x: 105, y: 100, z: 0 });
        });
        let (tx_b, rx_b) = async_std::channel::bounded::<Vec<u8>>(4);
        mgr.register_sender(id_b, tx_b);

        // Player C at (500, 500) — out of range
        let id_c = mgr.add_connection(addr(5003));
        mgr.with_connection_mut(id_c, |c| {
            for s in &[ConnectionState::LoginSeed, ConnectionState::Authenticating,
                       ConnectionState::ServerSelect, ConnectionState::GameLogin,
                       ConnectionState::InGame] {
                let _ = c.transition_to(*s);
            }
            c.set_position(Position { x: 500, y: 500, z: 0 });
        });
        let (tx_c, rx_c) = async_std::channel::bounded::<Vec<u8>>(4);
        mgr.register_sender(id_c, tx_c);

        mgr.broadcast_to_range(id_a, Position { x: 100, y: 100, z: 0 }, 18, vec![0x77]);

        assert!(rx_b.try_recv().is_ok(), "B should receive broadcast");
        assert!(rx_c.try_recv().is_err(), "C is out of range");
    }
}
