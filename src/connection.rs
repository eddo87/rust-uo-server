use std::fmt;
use std::net::SocketAddr;
use std::time::Instant;

use crate::character::Character;
use crate::movement::Position;
use crate::ticks;

/// Represents the stages a client connection moves through during the login flow.
///
/// The valid transition order is:
///   Connecting -> LoginSeed -> Authenticating -> ServerSelect -> GameLogin -> InGame
/// A transition to Disconnected is allowed from any state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Initial state when a TCP connection is accepted.
    Connecting,
    /// The client has sent the encrypted login seed packet (0xEF).
    LoginSeed,
    /// The client has sent account credentials (0x80) and we are verifying them.
    Authenticating,
    /// Authentication succeeded; the client is choosing a server from the list.
    ServerSelect,
    /// The client selected a server and sent the game login packet (0x91).
    GameLogin,
    /// The client is fully logged in and playing.
    InGame,
    /// The connection has been closed or dropped.
    Disconnected,
}

impl fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            ConnectionState::Connecting => "Connecting",
            ConnectionState::LoginSeed => "LoginSeed",
            ConnectionState::Authenticating => "Authenticating",
            ConnectionState::ServerSelect => "ServerSelect",
            ConnectionState::GameLogin => "GameLogin",
            ConnectionState::InGame => "InGame",
            ConnectionState::Disconnected => "Disconnected",
        };
        write!(f, "{}", label)
    }
}

impl ConnectionState {
    /// Returns the single valid *forward* successor for this state, if one exists.
    /// `Disconnected` and `InGame` have no forward successor.
    fn next_state(&self) -> Option<ConnectionState> {
        match self {
            ConnectionState::Connecting => Some(ConnectionState::LoginSeed),
            ConnectionState::LoginSeed => Some(ConnectionState::Authenticating),
            ConnectionState::Authenticating => Some(ConnectionState::ServerSelect),
            ConnectionState::ServerSelect => Some(ConnectionState::GameLogin),
            ConnectionState::GameLogin => Some(ConnectionState::InGame),
            ConnectionState::InGame => None,
            ConnectionState::Disconnected => None,
        }
    }

    /// Returns `true` when transitioning from `self` to `target` is allowed.
    ///
    /// Rules:
    /// - Any state may transition to `Disconnected`.
    /// - Otherwise only the single forward step (see `next_state`) is permitted.
    /// - A `Disconnected` connection cannot transition to anything.
    pub fn can_transition_to(&self, target: ConnectionState) -> bool {
        if *self == ConnectionState::Disconnected {
            return false;
        }
        if target == ConnectionState::Disconnected {
            return true;
        }
        self.next_state() == Some(target)
    }
}

/// Tracks the full state of a single client connection.
#[derive(Debug, Clone)]
pub struct Connection {
    pub id: u64,
    pub state: ConnectionState,
    pub address: SocketAddr,
    pub account_name: Option<String>,
    pub character_name: Option<String>,
    pub client_version: Option<(u8, u8, u8, u8)>,
    pub connected_at: i64,
    /// Current map position; `None` until the player enters the game world.
    pub position: Option<Position>,
    /// Current map id (0 = Felucca).
    pub map_id: u8,
    /// Last acknowledged movement sequence number.
    pub move_sequence: u8,
    /// Whether the player is currently in war mode.
    pub war_mode: bool,
    /// Unique serial number for this player's mobile, used in UO packets.
    pub serial: u32,
    /// Full character data (stats, HP, skills). Set on character select.
    pub character: Option<Character>,
    /// Serial of the mobile this player is currently targeting for combat.
    pub target_serial: Option<u32>,
    /// Timestamp of the last resolved swing, used to enforce the swing timer.
    pub last_swing: Option<Instant>,
    /// Serial of this player's backpack container (0 = not yet assigned).
    pub backpack_serial: u32,
    /// Serial of the ethereal horse statuette item inside the backpack.
    pub statuette_serial: u32,
    /// Virtual serial used as the mount item in 0x78 equipment layer 0x19.
    pub mount_item_serial: u32,
    /// Serial of this player's bank box container.
    pub bank_box_serial: u32,
    /// Whether the player is currently riding a mount.
    pub mounted: bool,
}

impl Connection {
    /// Create a new connection in the `Connecting` state.
    pub fn new(id: u64, address: SocketAddr) -> Connection {
        Connection {
            id,
            state: ConnectionState::Connecting,
            address,
            account_name: None,
            character_name: None,
            client_version: None,
            connected_at: ticks::current_ticks(),
            position: None,
            map_id: 0,
            move_sequence: 0,
            war_mode: false,
            serial: (id as u32).wrapping_add(1),
            character: None,
            target_serial: None,
            last_swing: None,
            backpack_serial: 0,
            statuette_serial: 0,
            mount_item_serial: 0,
            bank_box_serial: 0,
            mounted: false,
        }
    }

    /// Attempt to move this connection to `new_state`.
    ///
    /// Returns `Ok(())` on success, or `Err` with a descriptive message when the
    /// transition is not allowed.
    pub fn transition_to(&mut self, new_state: ConnectionState) -> Result<(), String> {
        if self.state.can_transition_to(new_state) {
            self.state = new_state;
            Ok(())
        } else {
            Err(format!(
                "Invalid state transition: {} -> {} (connection {})",
                self.state, new_state, self.id
            ))
        }
    }

    /// Returns `true` when the connection has reached the `InGame` state.
    pub fn is_in_game(&self) -> bool {
        self.state == ConnectionState::InGame
    }

    /// Returns `true` when the connection is in the `Disconnected` state.
    pub fn is_disconnected(&self) -> bool {
        self.state == ConnectionState::Disconnected
    }

    /// Set the client version extracted from the login seed packet.
    pub fn set_client_version(&mut self, major: u8, minor: u8, revision: u8, patch: u8) {
        self.client_version = Some((major, minor, revision, patch));
    }

    /// Set the account name extracted from a login packet.
    pub fn set_account_name(&mut self, name: String) {
        self.account_name = Some(name);
    }

    /// Set the character name once the player selects or creates a character.
    pub fn set_character_name(&mut self, name: String) {
        self.character_name = Some(name);
    }

    /// Set (or update) the player's current map position.
    pub fn set_position(&mut self, pos: Position) {
        self.position = Some(pos);
    }

    /// Record the last acknowledged movement sequence number.
    pub fn update_move_sequence(&mut self, seq: u8) {
        self.move_sequence = seq;
    }

    /// Set the player's war mode state.
    pub fn set_war_mode(&mut self, war: bool) {
        self.war_mode = war;
    }
}

impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Connection {{ id: {}, state: {}, address: {}, account: {} }}",
            self.id,
            self.state,
            self.address,
            self.account_name.as_deref().unwrap_or("<none>"),
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn test_addr() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12345)
    }

    // -- ConnectionState transition validation --------------------------------

    #[test]
    fn valid_forward_transitions() {
        use ConnectionState::*;
        let chain = [
            Connecting,
            LoginSeed,
            Authenticating,
            ServerSelect,
            GameLogin,
            InGame,
        ];
        for window in chain.windows(2) {
            assert!(
                window[0].can_transition_to(window[1]),
                "{} -> {} should be valid",
                window[0],
                window[1]
            );
        }
    }

    #[test]
    fn any_state_can_transition_to_disconnected() {
        use ConnectionState::*;
        let states = [
            Connecting,
            LoginSeed,
            Authenticating,
            ServerSelect,
            GameLogin,
            InGame,
        ];
        for state in &states {
            assert!(
                state.can_transition_to(Disconnected),
                "{} -> Disconnected should be valid",
                state
            );
        }
    }

    #[test]
    fn disconnected_cannot_transition_anywhere() {
        use ConnectionState::*;
        let targets = [
            Connecting,
            LoginSeed,
            Authenticating,
            ServerSelect,
            GameLogin,
            InGame,
            Disconnected,
        ];
        for target in &targets {
            assert!(
                !Disconnected.can_transition_to(*target),
                "Disconnected -> {} should be invalid",
                target
            );
        }
    }

    #[test]
    fn skipping_states_is_invalid() {
        use ConnectionState::*;
        assert!(!Connecting.can_transition_to(Authenticating));
        assert!(!Connecting.can_transition_to(InGame));
        assert!(!LoginSeed.can_transition_to(ServerSelect));
        assert!(!Authenticating.can_transition_to(InGame));
    }

    #[test]
    fn backward_transitions_are_invalid() {
        use ConnectionState::*;
        assert!(!LoginSeed.can_transition_to(Connecting));
        assert!(!Authenticating.can_transition_to(LoginSeed));
        assert!(!InGame.can_transition_to(Connecting));
        assert!(!ServerSelect.can_transition_to(Authenticating));
    }

    #[test]
    fn same_state_transition_is_invalid() {
        use ConnectionState::*;
        let states = [
            Connecting,
            LoginSeed,
            Authenticating,
            ServerSelect,
            GameLogin,
            InGame,
        ];
        for state in &states {
            assert!(
                !state.can_transition_to(*state),
                "{} -> {} (same state) should be invalid",
                state,
                state
            );
        }
    }

    // -- Connection struct tests ---------------------------------------------

    #[test]
    fn new_connection_starts_in_connecting() {
        let conn = Connection::new(1, test_addr());
        assert_eq!(conn.state, ConnectionState::Connecting);
        assert_eq!(conn.id, 1);
        assert_eq!(conn.address, test_addr());
        assert!(conn.account_name.is_none());
        assert!(conn.character_name.is_none());
        assert!(conn.client_version.is_none());
        assert!(!conn.is_in_game());
        assert!(!conn.is_disconnected());
    }

    #[test]
    fn full_login_flow_succeeds() {
        let mut conn = Connection::new(42, test_addr());

        assert!(conn.transition_to(ConnectionState::LoginSeed).is_ok());
        conn.set_client_version(7, 0, 95, 0);

        assert!(conn.transition_to(ConnectionState::Authenticating).is_ok());
        conn.set_account_name("testplayer".to_string());

        assert!(conn.transition_to(ConnectionState::ServerSelect).is_ok());
        assert!(conn.transition_to(ConnectionState::GameLogin).is_ok());
        assert!(conn.transition_to(ConnectionState::InGame).is_ok());

        assert!(conn.is_in_game());
        assert!(!conn.is_disconnected());
        assert_eq!(conn.client_version, Some((7, 0, 95, 0)));
        assert_eq!(conn.account_name.as_deref(), Some("testplayer"));
    }

    #[test]
    fn invalid_transition_returns_error() {
        let mut conn = Connection::new(1, test_addr());
        let result = conn.transition_to(ConnectionState::InGame);
        assert!(result.is_err());
        // State must remain unchanged after a failed transition.
        assert_eq!(conn.state, ConnectionState::Connecting);
    }

    #[test]
    fn disconnect_from_any_active_state() {
        for i in 0..5u8 {
            let mut conn = Connection::new(u64::from(i), test_addr());
            // Walk forward i steps.
            let states = [
                ConnectionState::LoginSeed,
                ConnectionState::Authenticating,
                ConnectionState::ServerSelect,
                ConnectionState::GameLogin,
                ConnectionState::InGame,
            ];
            for s in states.iter().take(i as usize) {
                conn.transition_to(*s).unwrap();
            }
            assert!(conn.transition_to(ConnectionState::Disconnected).is_ok());
            assert!(conn.is_disconnected());
        }
    }

    #[test]
    fn transition_after_disconnect_fails() {
        let mut conn = Connection::new(1, test_addr());
        conn.transition_to(ConnectionState::Disconnected).unwrap();
        assert!(conn.transition_to(ConnectionState::Connecting).is_err());
        assert!(conn.transition_to(ConnectionState::LoginSeed).is_err());
        assert!(conn.transition_to(ConnectionState::Disconnected).is_err());
    }

    #[test]
    fn set_character_name_works() {
        let mut conn = Connection::new(1, test_addr());
        conn.set_character_name("Lord British".to_string());
        assert_eq!(conn.character_name.as_deref(), Some("Lord British"));
    }

    #[test]
    fn display_formatting() {
        let conn = Connection::new(7, test_addr());
        let display = format!("{}", conn);
        assert!(display.contains("id: 7"));
        assert!(display.contains("Connecting"));
        assert!(display.contains("127.0.0.1:12345"));
    }

    #[test]
    fn connected_at_is_populated() {
        let before = ticks::current_ticks();
        let conn = Connection::new(1, test_addr());
        let after = ticks::current_ticks();
        assert!(conn.connected_at >= before);
        assert!(conn.connected_at <= after);
    }

    #[test]
    fn position_starts_none() {
        let conn = Connection::new(1, test_addr());
        assert!(conn.position.is_none());
        assert_eq!(conn.map_id, 0);
        assert_eq!(conn.move_sequence, 0);
    }

    #[test]
    fn set_position_works() {
        use crate::movement::Position;
        let mut conn = Connection::new(1, test_addr());
        let pos = Position { x: 1496, y: 1628, z: 10 };
        conn.set_position(pos);
        assert_eq!(conn.position, Some(pos));
    }

    #[test]
    fn move_sequence_updates() {
        let mut conn = Connection::new(1, test_addr());
        assert_eq!(conn.move_sequence, 0);
        conn.update_move_sequence(42);
        assert_eq!(conn.move_sequence, 42);
        conn.update_move_sequence(255);
        assert_eq!(conn.move_sequence, 255);
    }
}
