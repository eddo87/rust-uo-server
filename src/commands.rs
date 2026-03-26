use std::collections::HashMap;
use std::fmt;

/// Access levels for GM commands, ordered from least to most privileged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessLevel {
    Player,
    Counselor,
    GameMaster,
    Seer,
    Administrator,
    Owner,
}

impl AccessLevel {
    fn rank(self) -> u8 {
        match self {
            AccessLevel::Player => 0,
            AccessLevel::Counselor => 1,
            AccessLevel::GameMaster => 2,
            AccessLevel::Seer => 3,
            AccessLevel::Administrator => 4,
            AccessLevel::Owner => 5,
        }
    }
}

impl PartialOrd for AccessLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.rank().cmp(&other.rank()))
    }
}

impl Ord for AccessLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.rank().cmp(&other.rank())
    }
}

impl fmt::Display for AccessLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccessLevel::Player => write!(f, "Player"),
            AccessLevel::Counselor => write!(f, "Counselor"),
            AccessLevel::GameMaster => write!(f, "GameMaster"),
            AccessLevel::Seer => write!(f, "Seer"),
            AccessLevel::Administrator => write!(f, "Administrator"),
            AccessLevel::Owner => write!(f, "Owner"),
        }
    }
}

/// Arguments passed to a command handler.
#[derive(Debug, Clone)]
pub struct CommandArgs {
    /// The original, unparsed argument string.
    pub raw_args: String,
    /// Arguments split by whitespace.
    pub parsed_args: Vec<String>,
    /// Serial of the character that invoked the command.
    pub caller_serial: u32,
    /// Access level of the caller.
    pub caller_access: AccessLevel,
}

impl CommandArgs {
    pub fn new(raw_args: &str, caller_serial: u32, caller_access: AccessLevel) -> Self {
        let parsed_args = raw_args
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        CommandArgs {
            raw_args: raw_args.to_string(),
            parsed_args,
            caller_serial,
            caller_access,
        }
    }
}

/// Result of executing a command.
#[derive(Debug, Clone, PartialEq)]
pub enum CommandResult {
    /// Command executed successfully with an optional message.
    Success(String),
    /// Command failed with an error message.
    Failure(String),
    /// Caller does not have permission to run this command.
    AccessDenied,
    /// Command was not found in the registry.
    NotFound,
}

/// Type alias for command handler functions.
pub type CommandHandler = fn(&CommandArgs) -> CommandResult;

/// A registered command.
pub struct Command {
    /// Primary name used to invoke the command.
    pub name: String,
    /// Short description shown in help text.
    pub description: String,
    /// Minimum access level required to execute this command.
    pub access_level: AccessLevel,
    /// Function that executes the command logic.
    pub handler: CommandHandler,
    /// Alternative names that also invoke this command.
    pub aliases: Vec<String>,
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Command")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("access_level", &self.access_level)
            .field("aliases", &self.aliases)
            .finish()
    }
}

/// Registry that stores commands and resolves them by name (case-insensitive).
pub struct CommandRegistry {
    commands: HashMap<String, Command>,
    /// Maps lowercase alias -> lowercase primary name.
    alias_map: HashMap<String, String>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        CommandRegistry {
            commands: HashMap::new(),
            alias_map: HashMap::new(),
        }
    }

    /// Register a new command. Names and aliases are stored in lowercase.
    pub fn register(&mut self, command: Command) {
        let key = command.name.to_lowercase();
        for alias in &command.aliases {
            self.alias_map
                .insert(alias.to_lowercase(), key.clone());
        }
        self.commands.insert(key, command);
    }

    /// Look up a command by name or alias (case-insensitive).
    pub fn get(&self, name: &str) -> Option<&Command> {
        let lower = name.to_lowercase();
        if let Some(cmd) = self.commands.get(&lower) {
            return Some(cmd);
        }
        if let Some(primary) = self.alias_map.get(&lower) {
            return self.commands.get(primary);
        }
        None
    }

    /// Execute a command by name, checking access level.
    pub fn execute(&self, name: &str, args: &CommandArgs) -> CommandResult {
        match self.get(name) {
            Some(cmd) => {
                if args.caller_access >= cmd.access_level {
                    (cmd.handler)(args)
                } else {
                    CommandResult::AccessDenied
                }
            }
            None => CommandResult::NotFound,
        }
    }

    /// Return a sorted list of all registered commands.
    pub fn list_commands(&self) -> Vec<&Command> {
        let mut cmds: Vec<&Command> = self.commands.values().collect();
        cmds.sort_by(|a, b| a.name.cmp(&b.name));
        cmds
    }

    /// Return only the commands the given access level may use.
    pub fn list_commands_for(&self, access: AccessLevel) -> Vec<&Command> {
        let mut cmds: Vec<&Command> = self
            .commands
            .values()
            .filter(|c| access >= c.access_level)
            .collect();
        cmds.sort_by(|a, b| a.name.cmp(&b.name));
        cmds
    }

    /// Total number of registered primary commands.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Built-in command handlers
// ---------------------------------------------------------------------------

fn cmd_help(args: &CommandArgs) -> CommandResult {
    if args.parsed_args.is_empty() {
        CommandResult::Success("Available commands: help, where, go, kill, resurrect, add, set, ban, unban, save, broadcast, shutdown. Use 'help <command>' for details.".to_string())
    } else {
        CommandResult::Success(format!("Help for command '{}'.", args.parsed_args[0]))
    }
}

fn cmd_where(args: &CommandArgs) -> CommandResult {
    CommandResult::Success(format!(
        "Player 0x{:08X} location requested.",
        args.caller_serial
    ))
}

fn cmd_go(args: &CommandArgs) -> CommandResult {
    if args.parsed_args.len() < 2 {
        return CommandResult::Failure("Usage: go <x> <y> [z]".to_string());
    }
    CommandResult::Success(format!(
        "Teleporting to {} {}.",
        args.parsed_args[0], args.parsed_args[1]
    ))
}

fn cmd_kill(args: &CommandArgs) -> CommandResult {
    if args.parsed_args.is_empty() {
        return CommandResult::Failure("Usage: kill <target_serial>".to_string());
    }
    CommandResult::Success(format!("Killed target {}.", args.parsed_args[0]))
}

fn cmd_resurrect(args: &CommandArgs) -> CommandResult {
    if args.parsed_args.is_empty() {
        return CommandResult::Failure("Usage: resurrect <target_serial>".to_string());
    }
    CommandResult::Success(format!("Resurrected target {}.", args.parsed_args[0]))
}

fn cmd_add(args: &CommandArgs) -> CommandResult {
    if args.parsed_args.is_empty() {
        return CommandResult::Failure("Usage: add <item_id> [amount]".to_string());
    }
    CommandResult::Success(format!("Added item {}.", args.parsed_args[0]))
}

fn cmd_set(args: &CommandArgs) -> CommandResult {
    if args.parsed_args.len() < 2 {
        return CommandResult::Failure("Usage: set <property> <value>".to_string());
    }
    CommandResult::Success(format!(
        "Set {} to {}.",
        args.parsed_args[0], args.parsed_args[1]
    ))
}

fn cmd_ban(args: &CommandArgs) -> CommandResult {
    if args.parsed_args.is_empty() {
        return CommandResult::Failure("Usage: ban <account>".to_string());
    }
    CommandResult::Success(format!("Banned account {}.", args.parsed_args[0]))
}

fn cmd_unban(args: &CommandArgs) -> CommandResult {
    if args.parsed_args.is_empty() {
        return CommandResult::Failure("Usage: unban <account>".to_string());
    }
    CommandResult::Success(format!("Unbanned account {}.", args.parsed_args[0]))
}

fn cmd_save(_args: &CommandArgs) -> CommandResult {
    CommandResult::Success("World save initiated.".to_string())
}

fn cmd_broadcast(args: &CommandArgs) -> CommandResult {
    if args.raw_args.is_empty() {
        return CommandResult::Failure("Usage: broadcast <message>".to_string());
    }
    CommandResult::Success(format!("Broadcast: {}", args.raw_args))
}

fn cmd_shutdown(_args: &CommandArgs) -> CommandResult {
    CommandResult::Success("Server shutdown initiated.".to_string())
}

// ---------------------------------------------------------------------------
// Default registry with all built-in commands
// ---------------------------------------------------------------------------

/// Create a `CommandRegistry` pre-loaded with all built-in GM commands.
pub fn default_registry() -> CommandRegistry {
    let mut reg = CommandRegistry::new();

    reg.register(Command {
        name: "help".to_string(),
        description: "Show available commands or details for a specific command.".to_string(),
        access_level: AccessLevel::Player,
        handler: cmd_help,
        aliases: vec!["?".to_string(), "commands".to_string()],
    });

    reg.register(Command {
        name: "where".to_string(),
        description: "Display your current coordinates.".to_string(),
        access_level: AccessLevel::Counselor,
        handler: cmd_where,
        aliases: vec!["location".to_string()],
    });

    reg.register(Command {
        name: "go".to_string(),
        description: "Teleport to the specified coordinates.".to_string(),
        access_level: AccessLevel::Counselor,
        handler: cmd_go,
        aliases: vec!["teleport".to_string(), "goto".to_string()],
    });

    reg.register(Command {
        name: "kill".to_string(),
        description: "Kill the target character or mobile.".to_string(),
        access_level: AccessLevel::GameMaster,
        handler: cmd_kill,
        aliases: vec!["slay".to_string()],
    });

    reg.register(Command {
        name: "resurrect".to_string(),
        description: "Resurrect the target character.".to_string(),
        access_level: AccessLevel::GameMaster,
        handler: cmd_resurrect,
        aliases: vec!["res".to_string(), "ress".to_string()],
    });

    reg.register(Command {
        name: "add".to_string(),
        description: "Add an item or mobile to the world.".to_string(),
        access_level: AccessLevel::GameMaster,
        handler: cmd_add,
        aliases: vec!["create".to_string(), "spawn".to_string()],
    });

    reg.register(Command {
        name: "set".to_string(),
        description: "Set a property on the target object.".to_string(),
        access_level: AccessLevel::Seer,
        handler: cmd_set,
        aliases: vec![],
    });

    reg.register(Command {
        name: "ban".to_string(),
        description: "Ban an account from the server.".to_string(),
        access_level: AccessLevel::Administrator,
        handler: cmd_ban,
        aliases: vec![],
    });

    reg.register(Command {
        name: "unban".to_string(),
        description: "Remove a ban from an account.".to_string(),
        access_level: AccessLevel::Administrator,
        handler: cmd_unban,
        aliases: vec![],
    });

    reg.register(Command {
        name: "save".to_string(),
        description: "Force a world save.".to_string(),
        access_level: AccessLevel::Administrator,
        handler: cmd_save,
        aliases: vec!["worldsave".to_string()],
    });

    reg.register(Command {
        name: "broadcast".to_string(),
        description: "Send a message to all online players.".to_string(),
        access_level: AccessLevel::Seer,
        handler: cmd_broadcast,
        aliases: vec!["bc".to_string()],
    });

    reg.register(Command {
        name: "shutdown".to_string(),
        description: "Shut down the server.".to_string(),
        access_level: AccessLevel::Owner,
        handler: cmd_shutdown,
        aliases: vec![],
    });

    reg
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- AccessLevel ordering -----------------------------------------------

    #[test]
    fn access_level_ordering() {
        assert!(AccessLevel::Player < AccessLevel::Counselor);
        assert!(AccessLevel::Counselor < AccessLevel::GameMaster);
        assert!(AccessLevel::GameMaster < AccessLevel::Seer);
        assert!(AccessLevel::Seer < AccessLevel::Administrator);
        assert!(AccessLevel::Administrator < AccessLevel::Owner);
    }

    #[test]
    fn access_level_equality() {
        assert_eq!(AccessLevel::GameMaster, AccessLevel::GameMaster);
        assert_ne!(AccessLevel::Player, AccessLevel::Owner);
    }

    #[test]
    fn access_level_display() {
        assert_eq!(format!("{}", AccessLevel::Player), "Player");
        assert_eq!(format!("{}", AccessLevel::Owner), "Owner");
    }

    // -- CommandArgs --------------------------------------------------------

    #[test]
    fn command_args_parsing() {
        let args = CommandArgs::new("hello world 42", 0x1000, AccessLevel::Player);
        assert_eq!(args.raw_args, "hello world 42");
        assert_eq!(args.parsed_args, vec!["hello", "world", "42"]);
        assert_eq!(args.caller_serial, 0x1000);
        assert_eq!(args.caller_access, AccessLevel::Player);
    }

    #[test]
    fn command_args_empty() {
        let args = CommandArgs::new("", 0x0, AccessLevel::Player);
        assert!(args.parsed_args.is_empty());
    }

    #[test]
    fn command_args_extra_whitespace() {
        let args = CommandArgs::new("  foo   bar  ", 0x0, AccessLevel::Player);
        assert_eq!(args.parsed_args, vec!["foo", "bar"]);
    }

    // -- CommandRegistry basics ---------------------------------------------

    #[test]
    fn registry_starts_empty() {
        let reg = CommandRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn default_registry_has_12_commands() {
        let reg = default_registry();
        assert_eq!(reg.len(), 12);
    }

    #[test]
    fn lookup_by_primary_name() {
        let reg = default_registry();
        assert!(reg.get("help").is_some());
        assert!(reg.get("shutdown").is_some());
    }

    #[test]
    fn lookup_case_insensitive() {
        let reg = default_registry();
        assert!(reg.get("HELP").is_some());
        assert!(reg.get("Help").is_some());
        assert!(reg.get("ShUtDoWn").is_some());
    }

    #[test]
    fn lookup_by_alias() {
        let reg = default_registry();
        let cmd = reg.get("?").expect("alias '?' should resolve");
        assert_eq!(cmd.name, "help");

        let cmd = reg.get("teleport").expect("alias 'teleport' should resolve");
        assert_eq!(cmd.name, "go");

        let cmd = reg.get("res").expect("alias 'res' should resolve");
        assert_eq!(cmd.name, "resurrect");

        let cmd = reg.get("bc").expect("alias 'bc' should resolve");
        assert_eq!(cmd.name, "broadcast");

        let cmd = reg.get("worldsave").expect("alias 'worldsave' should resolve");
        assert_eq!(cmd.name, "save");
    }

    #[test]
    fn lookup_alias_case_insensitive() {
        let reg = default_registry();
        let cmd = reg.get("TELEPORT").expect("alias should be case-insensitive");
        assert_eq!(cmd.name, "go");
    }

    #[test]
    fn lookup_nonexistent_returns_none() {
        let reg = default_registry();
        assert!(reg.get("doesnotexist").is_none());
    }

    // -- Execution & access control -----------------------------------------

    #[test]
    fn execute_help_as_player() {
        let reg = default_registry();
        let args = CommandArgs::new("", 0x1, AccessLevel::Player);
        match reg.execute("help", &args) {
            CommandResult::Success(msg) => assert!(msg.contains("Available commands")),
            other => panic!("Expected Success, got {:?}", other),
        }
    }

    #[test]
    fn execute_help_with_topic() {
        let reg = default_registry();
        let args = CommandArgs::new("go", 0x1, AccessLevel::Player);
        match reg.execute("help", &args) {
            CommandResult::Success(msg) => assert!(msg.contains("go")),
            other => panic!("Expected Success, got {:?}", other),
        }
    }

    #[test]
    fn execute_denied_for_low_access() {
        let reg = default_registry();
        let args = CommandArgs::new("", 0x1, AccessLevel::Player);
        assert_eq!(reg.execute("shutdown", &args), CommandResult::AccessDenied);
    }

    #[test]
    fn execute_owner_can_run_shutdown() {
        let reg = default_registry();
        let args = CommandArgs::new("", 0x1, AccessLevel::Owner);
        match reg.execute("shutdown", &args) {
            CommandResult::Success(msg) => assert!(msg.contains("shutdown")),
            other => panic!("Expected Success, got {:?}", other),
        }
    }

    #[test]
    fn execute_not_found() {
        let reg = default_registry();
        let args = CommandArgs::new("", 0x1, AccessLevel::Owner);
        assert_eq!(reg.execute("nonexistent", &args), CommandResult::NotFound);
    }

    #[test]
    fn execute_via_alias() {
        let reg = default_registry();
        let args = CommandArgs::new("", 0x1, AccessLevel::Player);
        match reg.execute("?", &args) {
            CommandResult::Success(msg) => assert!(msg.contains("Available commands")),
            other => panic!("Expected Success, got {:?}", other),
        }
    }

    #[test]
    fn execute_go_missing_args() {
        let reg = default_registry();
        let args = CommandArgs::new("100", 0x1, AccessLevel::Counselor);
        match reg.execute("go", &args) {
            CommandResult::Failure(msg) => assert!(msg.contains("Usage")),
            other => panic!("Expected Failure, got {:?}", other),
        }
    }

    #[test]
    fn execute_go_success() {
        let reg = default_registry();
        let args = CommandArgs::new("100 200", 0x1, AccessLevel::GameMaster);
        match reg.execute("go", &args) {
            CommandResult::Success(msg) => {
                assert!(msg.contains("100"));
                assert!(msg.contains("200"));
            }
            other => panic!("Expected Success, got {:?}", other),
        }
    }

    #[test]
    fn execute_kill_missing_target() {
        let reg = default_registry();
        let args = CommandArgs::new("", 0x1, AccessLevel::GameMaster);
        match reg.execute("kill", &args) {
            CommandResult::Failure(msg) => assert!(msg.contains("Usage")),
            other => panic!("Expected Failure, got {:?}", other),
        }
    }

    #[test]
    fn execute_set_requires_two_args() {
        let reg = default_registry();
        let args = CommandArgs::new("str", 0x1, AccessLevel::Seer);
        match reg.execute("set", &args) {
            CommandResult::Failure(msg) => assert!(msg.contains("Usage")),
            other => panic!("Expected Failure, got {:?}", other),
        }
    }

    #[test]
    fn execute_broadcast_empty() {
        let reg = default_registry();
        let args = CommandArgs::new("", 0x1, AccessLevel::Seer);
        match reg.execute("broadcast", &args) {
            CommandResult::Failure(msg) => assert!(msg.contains("Usage")),
            other => panic!("Expected Failure, got {:?}", other),
        }
    }

    #[test]
    fn execute_broadcast_with_message() {
        let reg = default_registry();
        let args = CommandArgs::new("Server restarting in 5 minutes", 0x1, AccessLevel::Seer);
        match reg.execute("broadcast", &args) {
            CommandResult::Success(msg) => assert!(msg.contains("Server restarting")),
            other => panic!("Expected Success, got {:?}", other),
        }
    }

    #[test]
    fn execute_save_as_admin() {
        let reg = default_registry();
        let args = CommandArgs::new("", 0x1, AccessLevel::Administrator);
        match reg.execute("save", &args) {
            CommandResult::Success(msg) => assert!(msg.contains("save")),
            other => panic!("Expected Success, got {:?}", other),
        }
    }

    // -- list_commands helpers ----------------------------------------------

    #[test]
    fn list_commands_sorted() {
        let reg = default_registry();
        let cmds = reg.list_commands();
        let names: Vec<&str> = cmds.iter().map(|c| c.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn list_commands_for_player() {
        let reg = default_registry();
        let cmds = reg.list_commands_for(AccessLevel::Player);
        // Only help is available to a plain Player.
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name, "help");
    }

    #[test]
    fn list_commands_for_owner_shows_all() {
        let reg = default_registry();
        let cmds = reg.list_commands_for(AccessLevel::Owner);
        assert_eq!(cmds.len(), 12);
    }

    // -- where command shows serial -----------------------------------------

    #[test]
    fn execute_where_shows_serial() {
        let reg = default_registry();
        let args = CommandArgs::new("", 0xDEAD, AccessLevel::Counselor);
        match reg.execute("where", &args) {
            CommandResult::Success(msg) => assert!(msg.contains("0000DEAD")),
            other => panic!("Expected Success, got {:?}", other),
        }
    }

    // -- ban / unban --------------------------------------------------------

    #[test]
    fn execute_ban_success() {
        let reg = default_registry();
        let args = CommandArgs::new("griefer123", 0x1, AccessLevel::Administrator);
        match reg.execute("ban", &args) {
            CommandResult::Success(msg) => assert!(msg.contains("griefer123")),
            other => panic!("Expected Success, got {:?}", other),
        }
    }

    #[test]
    fn execute_unban_success() {
        let reg = default_registry();
        let args = CommandArgs::new("griefer123", 0x1, AccessLevel::Administrator);
        match reg.execute("unban", &args) {
            CommandResult::Success(msg) => assert!(msg.contains("griefer123")),
            other => panic!("Expected Success, got {:?}", other),
        }
    }

    // -- add / resurrect ----------------------------------------------------

    #[test]
    fn execute_add_success() {
        let reg = default_registry();
        let args = CommandArgs::new("0x1F3C", 0x1, AccessLevel::GameMaster);
        match reg.execute("add", &args) {
            CommandResult::Success(msg) => assert!(msg.contains("0x1F3C")),
            other => panic!("Expected Success, got {:?}", other),
        }
    }

    #[test]
    fn execute_resurrect_success() {
        let reg = default_registry();
        let args = CommandArgs::new("0xBEEF", 0x1, AccessLevel::GameMaster);
        match reg.execute("resurrect", &args) {
            CommandResult::Success(msg) => assert!(msg.contains("0xBEEF")),
            other => panic!("Expected Success, got {:?}", other),
        }
    }

    // -- Access boundary tests ----------------------------------------------

    #[test]
    fn counselor_can_run_counselor_commands() {
        let reg = default_registry();
        let args = CommandArgs::new("", 0x1, AccessLevel::Counselor);
        match reg.execute("where", &args) {
            CommandResult::Success(_) => {}
            other => panic!("Expected Success, got {:?}", other),
        }
    }

    #[test]
    fn player_cannot_run_counselor_commands() {
        let reg = default_registry();
        let args = CommandArgs::new("", 0x1, AccessLevel::Player);
        assert_eq!(reg.execute("where", &args), CommandResult::AccessDenied);
    }

    #[test]
    fn gm_cannot_run_admin_commands() {
        let reg = default_registry();
        let args = CommandArgs::new("someone", 0x1, AccessLevel::GameMaster);
        assert_eq!(reg.execute("ban", &args), CommandResult::AccessDenied);
    }
}
