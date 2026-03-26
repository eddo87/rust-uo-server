use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Guild type -- the three allegiance flavours from classic UO
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildType {
    Regular,
    Chaos,
    Order,
}

// ---------------------------------------------------------------------------
// Guild rank -- five tiers with promote / demote helpers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GuildRank {
    Ronin,
    Member,
    Emissary,
    Warlord,
    GuildMaster,
}

impl GuildRank {
    /// Promote to the next higher rank, if possible.
    /// Returns `None` when already at `GuildMaster`.
    pub fn promote(self) -> Option<GuildRank> {
        match self {
            GuildRank::Ronin => Some(GuildRank::Member),
            GuildRank::Member => Some(GuildRank::Emissary),
            GuildRank::Emissary => Some(GuildRank::Warlord),
            GuildRank::Warlord => Some(GuildRank::GuildMaster),
            GuildRank::GuildMaster => None,
        }
    }

    /// Demote to the next lower rank, if possible.
    /// Returns `None` when already at `Ronin`.
    pub fn demote(self) -> Option<GuildRank> {
        match self {
            GuildRank::GuildMaster => Some(GuildRank::Warlord),
            GuildRank::Warlord => Some(GuildRank::Emissary),
            GuildRank::Emissary => Some(GuildRank::Member),
            GuildRank::Member => Some(GuildRank::Ronin),
            GuildRank::Ronin => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Guild member
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct GuildMember {
    pub serial: u64,
    pub name: String,
    pub rank: GuildRank,
}

// ---------------------------------------------------------------------------
// War declaration between two guilds
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct WarDeclaration {
    pub target_guild_id: u64,
    pub declared_by_guild_id: u64,
}

// ---------------------------------------------------------------------------
// Guild
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Guild {
    pub id: u64,
    pub name: String,
    pub abbreviation: String,
    pub guild_type: GuildType,
    pub master: u64,
    pub members: Vec<GuildMember>,
    pub wars: Vec<WarDeclaration>,
    pub charter: String,
}

// ---------------------------------------------------------------------------
// Error type used by GuildManager operations
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub enum GuildError {
    GuildNotFound,
    MemberNotFound,
    MemberAlreadyInGuild,
    CannotPromote,
    CannotDemote,
    WarAlreadyDeclared,
    WarNotFound,
    NameAlreadyTaken,
}

// ---------------------------------------------------------------------------
// GuildManager -- central registry of guilds
// ---------------------------------------------------------------------------

pub struct GuildManager {
    guilds: HashMap<u64, Guild>,
    /// Maps a player serial to the guild they belong to.
    serial_to_guild: HashMap<u64, u64>,
    next_id: u64,
}

impl GuildManager {
    pub fn new() -> Self {
        Self {
            guilds: HashMap::new(),
            serial_to_guild: HashMap::new(),
            next_id: 1,
        }
    }

    // -- creation -----------------------------------------------------------

    /// Create a new guild.  The `master_serial` is automatically added as the
    /// first member with `GuildMaster` rank.
    pub fn create_guild(
        &mut self,
        name: &str,
        abbreviation: &str,
        guild_type: GuildType,
        master_serial: u64,
        master_name: &str,
        charter: &str,
    ) -> Result<u64, GuildError> {
        // Uniqueness check (case-insensitive).
        if self.find_by_name(name).is_some() {
            return Err(GuildError::NameAlreadyTaken);
        }

        // Master must not already belong to a guild.
        if self.serial_to_guild.contains_key(&master_serial) {
            return Err(GuildError::MemberAlreadyInGuild);
        }

        let id = self.next_id;
        self.next_id += 1;

        let master_member = GuildMember {
            serial: master_serial,
            name: master_name.to_string(),
            rank: GuildRank::GuildMaster,
        };

        let guild = Guild {
            id,
            name: name.to_string(),
            abbreviation: abbreviation.to_string(),
            guild_type,
            master: master_serial,
            members: vec![master_member],
            wars: Vec::new(),
            charter: charter.to_string(),
        };

        self.guilds.insert(id, guild);
        self.serial_to_guild.insert(master_serial, id);

        Ok(id)
    }

    // -- membership ---------------------------------------------------------

    pub fn add_member(
        &mut self,
        guild_id: u64,
        serial: u64,
        name: &str,
    ) -> Result<(), GuildError> {
        if self.serial_to_guild.contains_key(&serial) {
            return Err(GuildError::MemberAlreadyInGuild);
        }

        let guild = self.guilds.get_mut(&guild_id).ok_or(GuildError::GuildNotFound)?;

        guild.members.push(GuildMember {
            serial,
            name: name.to_string(),
            rank: GuildRank::Ronin,
        });

        self.serial_to_guild.insert(serial, guild_id);
        Ok(())
    }

    pub fn remove_member(
        &mut self,
        guild_id: u64,
        serial: u64,
    ) -> Result<(), GuildError> {
        let guild = self.guilds.get_mut(&guild_id).ok_or(GuildError::GuildNotFound)?;

        let idx = guild
            .members
            .iter()
            .position(|m| m.serial == serial)
            .ok_or(GuildError::MemberNotFound)?;

        guild.members.remove(idx);
        self.serial_to_guild.remove(&serial);
        Ok(())
    }

    // -- rank changes -------------------------------------------------------

    pub fn promote(
        &mut self,
        guild_id: u64,
        serial: u64,
    ) -> Result<GuildRank, GuildError> {
        let guild = self.guilds.get_mut(&guild_id).ok_or(GuildError::GuildNotFound)?;
        let member = guild
            .members
            .iter_mut()
            .find(|m| m.serial == serial)
            .ok_or(GuildError::MemberNotFound)?;

        let new_rank = member.rank.promote().ok_or(GuildError::CannotPromote)?;
        member.rank = new_rank;
        Ok(new_rank)
    }

    pub fn demote(
        &mut self,
        guild_id: u64,
        serial: u64,
    ) -> Result<GuildRank, GuildError> {
        let guild = self.guilds.get_mut(&guild_id).ok_or(GuildError::GuildNotFound)?;
        let member = guild
            .members
            .iter_mut()
            .find(|m| m.serial == serial)
            .ok_or(GuildError::MemberNotFound)?;

        let new_rank = member.rank.demote().ok_or(GuildError::CannotDemote)?;
        member.rank = new_rank;
        Ok(new_rank)
    }

    // -- wars ---------------------------------------------------------------

    pub fn declare_war(
        &mut self,
        guild_id: u64,
        target_guild_id: u64,
    ) -> Result<(), GuildError> {
        // Both guilds must exist.
        if !self.guilds.contains_key(&target_guild_id) {
            return Err(GuildError::GuildNotFound);
        }

        let guild = self.guilds.get_mut(&guild_id).ok_or(GuildError::GuildNotFound)?;

        // Don't allow duplicate declarations.
        if guild.wars.iter().any(|w| w.target_guild_id == target_guild_id) {
            return Err(GuildError::WarAlreadyDeclared);
        }

        guild.wars.push(WarDeclaration {
            target_guild_id,
            declared_by_guild_id: guild_id,
        });

        Ok(())
    }

    pub fn end_war(
        &mut self,
        guild_id: u64,
        target_guild_id: u64,
    ) -> Result<(), GuildError> {
        let guild = self.guilds.get_mut(&guild_id).ok_or(GuildError::GuildNotFound)?;

        let idx = guild
            .wars
            .iter()
            .position(|w| w.target_guild_id == target_guild_id)
            .ok_or(GuildError::WarNotFound)?;

        guild.wars.remove(idx);
        Ok(())
    }

    // -- lookup -------------------------------------------------------------

    /// Case-insensitive guild lookup by name.
    pub fn find_by_name(&self, name: &str) -> Option<&Guild> {
        let needle = name.to_lowercase();
        self.guilds.values().find(|g| g.name.to_lowercase() == needle)
    }

    /// Look up which guild a player serial belongs to.
    pub fn guild_for_serial(&self, serial: u64) -> Option<&Guild> {
        self.serial_to_guild
            .get(&serial)
            .and_then(|gid| self.guilds.get(gid))
    }

    /// Get a guild by its id.
    pub fn get_guild(&self, guild_id: u64) -> Option<&Guild> {
        self.guilds.get(&guild_id)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- helpers ------------------------------------------------------------

    fn make_manager_with_guild() -> (GuildManager, u64) {
        let mut mgr = GuildManager::new();
        let gid = mgr
            .create_guild("Knights", "KN", GuildType::Order, 100, "Arthur", "Honor!")
            .unwrap();
        (mgr, gid)
    }

    // -- GuildRank promote / demote -----------------------------------------

    #[test]
    fn rank_promote_full_chain() {
        let mut rank = GuildRank::Ronin;
        let expected = [
            GuildRank::Member,
            GuildRank::Emissary,
            GuildRank::Warlord,
            GuildRank::GuildMaster,
        ];
        for exp in &expected {
            rank = rank.promote().unwrap();
            assert_eq!(rank, *exp);
        }
        assert!(rank.promote().is_none());
    }

    #[test]
    fn rank_demote_full_chain() {
        let mut rank = GuildRank::GuildMaster;
        let expected = [
            GuildRank::Warlord,
            GuildRank::Emissary,
            GuildRank::Member,
            GuildRank::Ronin,
        ];
        for exp in &expected {
            rank = rank.demote().unwrap();
            assert_eq!(rank, *exp);
        }
        assert!(rank.demote().is_none());
    }

    // -- create_guild -------------------------------------------------------

    #[test]
    fn create_guild_basic() {
        let (mgr, gid) = make_manager_with_guild();
        let guild = mgr.get_guild(gid).unwrap();

        assert_eq!(guild.name, "Knights");
        assert_eq!(guild.abbreviation, "KN");
        assert_eq!(guild.guild_type, GuildType::Order);
        assert_eq!(guild.master, 100);
        assert_eq!(guild.members.len(), 1);
        assert_eq!(guild.members[0].rank, GuildRank::GuildMaster);
        assert_eq!(guild.charter, "Honor!");
    }

    #[test]
    fn create_guild_duplicate_name() {
        let (mut mgr, _) = make_manager_with_guild();
        let result =
            mgr.create_guild("knights", "KN2", GuildType::Regular, 200, "Bob", "");
        assert_eq!(result, Err(GuildError::NameAlreadyTaken));
    }

    #[test]
    fn create_guild_master_already_in_guild() {
        let (mut mgr, _) = make_manager_with_guild();
        let result =
            mgr.create_guild("OtherGuild", "OG", GuildType::Chaos, 100, "Arthur", "");
        assert_eq!(result, Err(GuildError::MemberAlreadyInGuild));
    }

    // -- add / remove member ------------------------------------------------

    #[test]
    fn add_and_remove_member() {
        let (mut mgr, gid) = make_manager_with_guild();

        mgr.add_member(gid, 200, "Lancelot").unwrap();
        assert_eq!(mgr.get_guild(gid).unwrap().members.len(), 2);
        assert!(mgr.guild_for_serial(200).is_some());

        mgr.remove_member(gid, 200).unwrap();
        assert_eq!(mgr.get_guild(gid).unwrap().members.len(), 1);
        assert!(mgr.guild_for_serial(200).is_none());
    }

    #[test]
    fn add_member_already_in_guild() {
        let (mut mgr, gid) = make_manager_with_guild();
        mgr.add_member(gid, 200, "Lancelot").unwrap();
        let result = mgr.add_member(gid, 200, "Lancelot");
        assert_eq!(result, Err(GuildError::MemberAlreadyInGuild));
    }

    #[test]
    fn add_member_guild_not_found() {
        let (mut mgr, _) = make_manager_with_guild();
        let result = mgr.add_member(999, 200, "Ghost");
        assert_eq!(result, Err(GuildError::GuildNotFound));
    }

    #[test]
    fn remove_member_not_found() {
        let (mut mgr, gid) = make_manager_with_guild();
        let result = mgr.remove_member(gid, 999);
        assert_eq!(result, Err(GuildError::MemberNotFound));
    }

    // -- promote / demote ---------------------------------------------------

    #[test]
    fn promote_member() {
        let (mut mgr, gid) = make_manager_with_guild();
        mgr.add_member(gid, 200, "Lancelot").unwrap();

        let rank = mgr.promote(gid, 200).unwrap();
        assert_eq!(rank, GuildRank::Member);
    }

    #[test]
    fn demote_member() {
        let (mut mgr, gid) = make_manager_with_guild();
        mgr.add_member(gid, 200, "Lancelot").unwrap();
        mgr.promote(gid, 200).unwrap(); // Ronin -> Member

        let rank = mgr.demote(gid, 200).unwrap();
        assert_eq!(rank, GuildRank::Ronin);
    }

    #[test]
    fn promote_at_max_rank() {
        let (mut mgr, gid) = make_manager_with_guild();
        // serial 100 is already GuildMaster
        let result = mgr.promote(gid, 100);
        assert_eq!(result, Err(GuildError::CannotPromote));
    }

    #[test]
    fn demote_at_min_rank() {
        let (mut mgr, gid) = make_manager_with_guild();
        mgr.add_member(gid, 200, "Lancelot").unwrap();
        // Lancelot starts as Ronin
        let result = mgr.demote(gid, 200);
        assert_eq!(result, Err(GuildError::CannotDemote));
    }

    // -- wars ---------------------------------------------------------------

    #[test]
    fn declare_and_end_war() {
        let mut mgr = GuildManager::new();
        let g1 = mgr
            .create_guild("Alpha", "AL", GuildType::Regular, 1, "A", "")
            .unwrap();
        let g2 = mgr
            .create_guild("Beta", "BE", GuildType::Chaos, 2, "B", "")
            .unwrap();

        mgr.declare_war(g1, g2).unwrap();
        assert_eq!(mgr.get_guild(g1).unwrap().wars.len(), 1);

        mgr.end_war(g1, g2).unwrap();
        assert!(mgr.get_guild(g1).unwrap().wars.is_empty());
    }

    #[test]
    fn declare_war_duplicate() {
        let mut mgr = GuildManager::new();
        let g1 = mgr
            .create_guild("Alpha", "AL", GuildType::Regular, 1, "A", "")
            .unwrap();
        let g2 = mgr
            .create_guild("Beta", "BE", GuildType::Chaos, 2, "B", "")
            .unwrap();

        mgr.declare_war(g1, g2).unwrap();
        let result = mgr.declare_war(g1, g2);
        assert_eq!(result, Err(GuildError::WarAlreadyDeclared));
    }

    #[test]
    fn end_war_not_found() {
        let (mut mgr, gid) = make_manager_with_guild();
        let result = mgr.end_war(gid, 999);
        assert_eq!(result, Err(GuildError::WarNotFound));
    }

    #[test]
    fn declare_war_target_not_found() {
        let (mut mgr, gid) = make_manager_with_guild();
        let result = mgr.declare_war(gid, 999);
        assert_eq!(result, Err(GuildError::GuildNotFound));
    }

    // -- find_by_name -------------------------------------------------------

    #[test]
    fn find_by_name_case_insensitive() {
        let (mgr, _) = make_manager_with_guild();

        assert!(mgr.find_by_name("Knights").is_some());
        assert!(mgr.find_by_name("KNIGHTS").is_some());
        assert!(mgr.find_by_name("knights").is_some());
        assert!(mgr.find_by_name("NoSuchGuild").is_none());
    }

    // -- serial_to_guild index ----------------------------------------------

    #[test]
    fn serial_to_guild_lookup() {
        let (mgr, gid) = make_manager_with_guild();
        let found = mgr.guild_for_serial(100).unwrap();
        assert_eq!(found.id, gid);
        assert!(mgr.guild_for_serial(999).is_none());
    }
}
