use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub shard: ShardConfig,
    #[serde(default)]
    pub game: GameConfig,
}

#[derive(Debug, Deserialize)]
pub struct NetworkConfig {
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
}

#[derive(Debug, Deserialize)]
pub struct ShardConfig {
    #[serde(default = "default_shard_name")]
    pub name: String,
    #[serde(default = "default_max_characters")]
    pub max_characters: u8,
}

#[derive(Debug, Deserialize)]
pub struct GameConfig {
    #[serde(default = "default_starting_city")]
    pub starting_city: String,
    #[serde(default = "default_max_hitpoints")]
    pub max_hitpoints: i16,
}

// Default value functions for serde
fn default_bind_address() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    2593
}

fn default_max_connections() -> usize {
    100
}

fn default_shard_name() -> String {
    "My Shard".to_string()
}

fn default_max_characters() -> u8 {
    7
}

fn default_starting_city() -> String {
    "Britain".to_string()
}

fn default_max_hitpoints() -> i16 {
    100
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            network: NetworkConfig::default(),
            shard: ShardConfig::default(),
            game: GameConfig::default(),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        NetworkConfig {
            bind_address: default_bind_address(),
            port: default_port(),
            max_connections: default_max_connections(),
        }
    }
}

impl Default for ShardConfig {
    fn default() -> Self {
        ShardConfig {
            name: default_shard_name(),
            max_characters: default_max_characters(),
        }
    }
}

impl Default for GameConfig {
    fn default() -> Self {
        GameConfig {
            starting_city: default_starting_city(),
            max_hitpoints: default_max_hitpoints(),
        }
    }
}

impl ServerConfig {
    /// Load configuration from "server.toml" if it exists, otherwise use defaults.
    ///
    /// If the file exists but contains invalid TOML, an error is returned.
    /// Missing fields in the file will be filled with default values.
    pub fn load() -> Result<ServerConfig, Box<dyn std::error::Error>> {
        let path = Path::new("server.toml");
        if path.exists() {
            let contents = fs::read_to_string(path)?;
            let config: ServerConfig = toml::from_str(&contents)?;
            Ok(config)
        } else {
            Ok(ServerConfig::default())
        }
    }

    /// Returns the full socket address string (e.g. "127.0.0.1:2593").
    pub fn socket_address(&self) -> String {
        format!("{}:{}", self.network.bind_address, self.network.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_network_config() {
        let config = NetworkConfig::default();
        assert_eq!(config.bind_address, "127.0.0.1");
        assert_eq!(config.port, 2593);
        assert_eq!(config.max_connections, 100);
    }

    #[test]
    fn test_default_shard_config() {
        let config = ShardConfig::default();
        assert_eq!(config.name, "My Shard");
        assert_eq!(config.max_characters, 7);
    }

    #[test]
    fn test_default_game_config() {
        let config = GameConfig::default();
        assert_eq!(config.starting_city, "Britain");
        assert_eq!(config.max_hitpoints, 100);
    }

    #[test]
    fn test_default_server_config() {
        let config = ServerConfig::default();
        assert_eq!(config.network.bind_address, "127.0.0.1");
        assert_eq!(config.network.port, 2593);
        assert_eq!(config.network.max_connections, 100);
        assert_eq!(config.shard.name, "My Shard");
        assert_eq!(config.shard.max_characters, 7);
        assert_eq!(config.game.starting_city, "Britain");
        assert_eq!(config.game.max_hitpoints, 100);
    }

    #[test]
    fn test_socket_address() {
        let config = ServerConfig::default();
        assert_eq!(config.socket_address(), "127.0.0.1:2593");
    }

    #[test]
    fn test_parse_full_toml() {
        let toml_str = r#"
[network]
bind_address = "0.0.0.0"
port = 3000
max_connections = 50

[shard]
name = "Test Shard"
max_characters = 5

[game]
starting_city = "Trinsic"
max_hitpoints = 200
"#;
        let config: ServerConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.network.bind_address, "0.0.0.0");
        assert_eq!(config.network.port, 3000);
        assert_eq!(config.network.max_connections, 50);
        assert_eq!(config.shard.name, "Test Shard");
        assert_eq!(config.shard.max_characters, 5);
        assert_eq!(config.game.starting_city, "Trinsic");
        assert_eq!(config.game.max_hitpoints, 200);
    }

    #[test]
    fn test_parse_partial_toml_uses_defaults() {
        let toml_str = r#"
[network]
port = 9999
"#;
        let config: ServerConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.network.bind_address, "127.0.0.1");
        assert_eq!(config.network.port, 9999);
        assert_eq!(config.network.max_connections, 100);
        // shard and game sections missing entirely => defaults
        assert_eq!(config.shard.name, "My Shard");
        assert_eq!(config.shard.max_characters, 7);
        assert_eq!(config.game.starting_city, "Britain");
        assert_eq!(config.game.max_hitpoints, 100);
    }

    #[test]
    fn test_parse_empty_toml_uses_defaults() {
        let toml_str = "";
        let config: ServerConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.network.bind_address, "127.0.0.1");
        assert_eq!(config.network.port, 2593);
        assert_eq!(config.shard.name, "My Shard");
        assert_eq!(config.game.starting_city, "Britain");
    }

    #[test]
    fn test_load_returns_defaults_when_no_file() {
        // When run from a temp directory with no server.toml, load() should return defaults
        let config = ServerConfig::load().unwrap();
        assert_eq!(config.network.port, 2593);
        assert_eq!(config.shard.name, "My Shard");
    }
}
