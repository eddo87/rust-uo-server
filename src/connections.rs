use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use crate::connection::Connection;

/// Thread-safe manager for all active client connections.
///
/// Internally backed by `Arc<Mutex<HashMap<u64, Connection>>>` so it can be
/// cheaply cloned and shared across async tasks.
#[derive(Debug, Clone)]
pub struct ConnectionManager {
    connections: Arc<Mutex<HashMap<u64, Connection>>>,
    next_id: Arc<Mutex<u64>>,
}

impl ConnectionManager {
    /// Create a new, empty connection manager.
    pub fn new() -> ConnectionManager {
        ConnectionManager {
            connections: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
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
        self.connections.lock().unwrap().remove(&id)
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
}
