use std::collections::HashMap;

/// Maximum number of members allowed in a single party.
const MAX_PARTY_SIZE: usize = 10;

/// Represents a single member within a party.
#[derive(Debug, Clone, PartialEq)]
pub struct PartyMember {
    pub serial: u32,
}

impl PartyMember {
    pub fn new(serial: u32) -> Self {
        Self { serial }
    }
}

/// Represents a party (group) of players.
#[derive(Debug, Clone)]
pub struct Party {
    pub id: u64,
    pub leader: u32,
    pub members: Vec<PartyMember>,
    pub candidates: Vec<u32>,
    pub loot_sharing: bool,
}

impl Party {
    pub fn new(id: u64, leader_serial: u32) -> Self {
        Self {
            id,
            leader: leader_serial,
            members: vec![PartyMember::new(leader_serial)],
            candidates: Vec::new(),
            loot_sharing: false,
        }
    }

    /// Returns true if the party has reached its maximum capacity.
    pub fn is_full(&self) -> bool {
        self.members.len() >= MAX_PARTY_SIZE
    }

    /// Returns true if the given serial is the party leader.
    pub fn is_leader(&self, serial: u32) -> bool {
        self.leader == serial
    }

    /// Returns true if the given serial is a member of this party.
    pub fn has_member(&self, serial: u32) -> bool {
        self.members.iter().any(|m| m.serial == serial)
    }

    /// Returns true if the given serial is a pending candidate.
    pub fn has_candidate(&self, serial: u32) -> bool {
        self.candidates.contains(&serial)
    }
}

/// Error types for party operations.
#[derive(Debug, Clone, PartialEq)]
pub enum PartyError {
    PartyNotFound,
    PartyFull,
    NotLeader,
    AlreadyInParty,
    NotInParty,
    NotCandidate,
    CannotRemoveLastMember,
    AlreadyCandidate,
}

/// Manages all active parties on the server.
pub struct PartyManager {
    parties: HashMap<u64, Party>,
    serial_to_party: HashMap<u32, u64>,
    next_id: u64,
}

impl PartyManager {
    pub fn new() -> Self {
        Self {
            parties: HashMap::new(),
            serial_to_party: HashMap::new(),
            next_id: 1,
        }
    }

    /// Returns the party a player belongs to, if any.
    pub fn get_party_for_player(&self, serial: u32) -> Option<&Party> {
        let party_id = self.serial_to_party.get(&serial)?;
        self.parties.get(party_id)
    }

    /// Creates a new party with the given serial as leader. Returns the party id.
    pub fn create_party(&mut self, leader_serial: u32) -> Result<u64, PartyError> {
        if self.serial_to_party.contains_key(&leader_serial) {
            return Err(PartyError::AlreadyInParty);
        }

        let id = self.next_id;
        self.next_id += 1;

        let party = Party::new(id, leader_serial);
        self.parties.insert(id, party);
        self.serial_to_party.insert(leader_serial, id);

        Ok(id)
    }

    /// The party leader invites a player. Adds them to the candidate list.
    pub fn invite(
        &mut self,
        leader_serial: u32,
        target_serial: u32,
    ) -> Result<u64, PartyError> {
        // If the leader has no party yet, create one first
        let party_id = if let Some(&id) = self.serial_to_party.get(&leader_serial) {
            id
        } else {
            self.create_party(leader_serial)?
        };

        if self.serial_to_party.contains_key(&target_serial) {
            return Err(PartyError::AlreadyInParty);
        }

        let party = self.parties.get_mut(&party_id).ok_or(PartyError::PartyNotFound)?;

        if !party.is_leader(leader_serial) {
            return Err(PartyError::NotLeader);
        }

        if party.is_full() {
            return Err(PartyError::PartyFull);
        }

        if party.has_candidate(target_serial) {
            return Err(PartyError::AlreadyCandidate);
        }

        party.candidates.push(target_serial);
        Ok(party_id)
    }

    /// A candidate accepts the party invitation and becomes a full member.
    pub fn accept_invite(&mut self, candidate_serial: u32, party_id: u64) -> Result<(), PartyError> {
        let party = self.parties.get_mut(&party_id).ok_or(PartyError::PartyNotFound)?;

        if !party.has_candidate(candidate_serial) {
            return Err(PartyError::NotCandidate);
        }

        if party.is_full() {
            // Remove from candidates since they can't join
            party.candidates.retain(|&s| s != candidate_serial);
            return Err(PartyError::PartyFull);
        }

        party.candidates.retain(|&s| s != candidate_serial);
        party.members.push(PartyMember::new(candidate_serial));
        self.serial_to_party.insert(candidate_serial, party_id);

        Ok(())
    }

    /// A candidate declines the party invitation.
    pub fn decline_invite(
        &mut self,
        candidate_serial: u32,
        party_id: u64,
    ) -> Result<(), PartyError> {
        let party = self.parties.get_mut(&party_id).ok_or(PartyError::PartyNotFound)?;

        if !party.has_candidate(candidate_serial) {
            return Err(PartyError::NotCandidate);
        }

        party.candidates.retain(|&s| s != candidate_serial);
        Ok(())
    }

    /// Removes a member from the party. Only the leader or the member themselves
    /// may remove. If the leader is removed, leadership transfers to the next
    /// member. If the party drops to zero real members, it is disbanded.
    pub fn remove_member(
        &mut self,
        requester_serial: u32,
        target_serial: u32,
    ) -> Result<(), PartyError> {
        let party_id = *self
            .serial_to_party
            .get(&target_serial)
            .ok_or(PartyError::NotInParty)?;

        let party = self.parties.get(&party_id).ok_or(PartyError::PartyNotFound)?;

        // Only the leader or the member themselves can remove
        let is_self_remove = requester_serial == target_serial;
        if !is_self_remove && !party.is_leader(requester_serial) {
            return Err(PartyError::NotLeader);
        }

        let party = self.parties.get_mut(&party_id).unwrap();
        party.members.retain(|m| m.serial != target_serial);
        self.serial_to_party.remove(&target_serial);

        if party.members.is_empty() {
            // Disband if no members left
            self.parties.remove(&party_id);
        } else if party.leader == target_serial {
            // Transfer leadership to the first remaining member
            party.leader = party.members[0].serial;
        }

        Ok(())
    }

    /// Sets a new leader for the party. Only the current leader may do this.
    pub fn set_leader(
        &mut self,
        current_leader: u32,
        new_leader: u32,
    ) -> Result<(), PartyError> {
        let party_id = *self
            .serial_to_party
            .get(&current_leader)
            .ok_or(PartyError::NotInParty)?;

        let party = self.parties.get_mut(&party_id).ok_or(PartyError::PartyNotFound)?;

        if !party.is_leader(current_leader) {
            return Err(PartyError::NotLeader);
        }

        if !party.has_member(new_leader) {
            return Err(PartyError::NotInParty);
        }

        party.leader = new_leader;
        Ok(())
    }

    /// Disbands the party entirely. Only the leader may disband.
    pub fn disband(&mut self, leader_serial: u32) -> Result<Vec<u32>, PartyError> {
        let party_id = *self
            .serial_to_party
            .get(&leader_serial)
            .ok_or(PartyError::NotInParty)?;

        let party = self.parties.get(&party_id).ok_or(PartyError::PartyNotFound)?;

        if !party.is_leader(leader_serial) {
            return Err(PartyError::NotLeader);
        }

        let member_serials: Vec<u32> = party.members.iter().map(|m| m.serial).collect();

        for &serial in &member_serials {
            self.serial_to_party.remove(&serial);
        }
        self.parties.remove(&party_id);

        Ok(member_serials)
    }
}

// ---------------------------------------------------------------------------
// Party packets -- UO packet 0xBF (General Information) sub-command 0x06
// ---------------------------------------------------------------------------
pub mod packets {
    use byteorder::{BigEndian, WriteBytesExt};

    /// Builds the outer 0xBF wrapper around a sub-command payload.
    fn wrap_bf(sub_payload: &[u8]) -> Vec<u8> {
        // 0xBF | length (u16) | sub-command (u16) | payload
        let total_len: u16 = 1 + 2 + 2 + sub_payload.len() as u16; // id + len + subcmd + data
        let mut buf = Vec::with_capacity(total_len as usize);
        buf.push(0xBF);                                   // packet id
        buf.write_u16::<BigEndian>(total_len).unwrap();    // packet length
        buf.write_u16::<BigEndian>(0x0006).unwrap();       // sub-command: party system
        buf.extend_from_slice(sub_payload);
        buf
    }

    /// Sub-command 0x01 -- Add a member to the party (server -> client).
    /// Tells all party members the updated member list.
    pub fn party_add_member(member_serials: &[u32]) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.push(0x01); // sub-sub command: add member
        payload.push(member_serials.len() as u8);
        for &serial in member_serials {
            payload.write_u32::<BigEndian>(serial).unwrap();
        }
        wrap_bf(&payload)
    }

    /// Sub-command 0x02 -- Remove a member from the party (server -> client).
    /// Sends the removed serial followed by the updated member list.
    pub fn party_remove_member(removed_serial: u32, remaining_serials: &[u32]) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.push(0x02); // sub-sub command: remove member
        payload.push(remaining_serials.len() as u8);
        payload.write_u32::<BigEndian>(removed_serial).unwrap();
        for &serial in remaining_serials {
            payload.write_u32::<BigEndian>(serial).unwrap();
        }
        wrap_bf(&payload)
    }

    /// Sub-command 0x07 -- Party invitation (server -> client).
    /// Sent to the target player to ask them to join.
    pub fn party_invite(leader_serial: u32) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.push(0x07); // sub-sub command: invite
        payload.write_u32::<BigEndian>(leader_serial).unwrap();
        wrap_bf(&payload)
    }

    /// Sub-command 0x09 -- Accept invite (client -> server acknowledgement).
    pub fn party_accept(leader_serial: u32) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.push(0x09); // sub-sub command: accept
        payload.write_u32::<BigEndian>(leader_serial).unwrap();
        wrap_bf(&payload)
    }

    /// Sub-command 0x08 -- Decline invite (client -> server).
    pub fn party_decline(leader_serial: u32) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.push(0x08); // sub-sub command: decline
        payload.write_u32::<BigEndian>(leader_serial).unwrap();
        wrap_bf(&payload)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    // -- PartyMember ---------------------------------------------------------

    #[test]
    fn party_member_new() {
        let m = PartyMember::new(0xAABBCCDD);
        assert_eq!(m.serial, 0xAABBCCDD);
    }

    // -- Party ---------------------------------------------------------------

    #[test]
    fn party_new_has_leader_as_member() {
        let p = Party::new(1, 100);
        assert_eq!(p.leader, 100);
        assert_eq!(p.members.len(), 1);
        assert!(p.has_member(100));
        assert!(!p.loot_sharing);
    }

    #[test]
    fn party_is_full_at_max() {
        let mut p = Party::new(1, 1);
        for i in 2..=MAX_PARTY_SIZE as u32 {
            p.members.push(PartyMember::new(i));
        }
        assert!(p.is_full());
    }

    #[test]
    fn party_is_leader_check() {
        let p = Party::new(1, 42);
        assert!(p.is_leader(42));
        assert!(!p.is_leader(99));
    }

    #[test]
    fn party_candidate_tracking() {
        let mut p = Party::new(1, 1);
        assert!(!p.has_candidate(10));
        p.candidates.push(10);
        assert!(p.has_candidate(10));
    }

    // -- PartyManager: create_party ------------------------------------------

    #[test]
    fn create_party_success() {
        let mut mgr = PartyManager::new();
        let id = mgr.create_party(100).unwrap();
        assert_eq!(id, 1);
        let party = mgr.get_party_for_player(100).unwrap();
        assert_eq!(party.leader, 100);
        assert_eq!(party.members.len(), 1);
    }

    #[test]
    fn create_party_already_in_party() {
        let mut mgr = PartyManager::new();
        mgr.create_party(100).unwrap();
        assert_eq!(mgr.create_party(100), Err(PartyError::AlreadyInParty));
    }

    // -- PartyManager: invite ------------------------------------------------

    #[test]
    fn invite_creates_party_if_none() {
        let mut mgr = PartyManager::new();
        let id = mgr.invite(100, 200).unwrap();
        let party = mgr.parties.get(&id).unwrap();
        assert!(party.has_candidate(200));
        assert!(party.has_member(100));
    }

    #[test]
    fn invite_fails_if_target_already_in_party() {
        let mut mgr = PartyManager::new();
        mgr.create_party(200).unwrap();
        mgr.create_party(100).unwrap();
        assert_eq!(mgr.invite(100, 200), Err(PartyError::AlreadyInParty));
    }

    #[test]
    fn invite_fails_if_not_leader() {
        let mut mgr = PartyManager::new();
        let id = mgr.create_party(100).unwrap();
        mgr.accept_invite(200, id).ok(); // won't work because not candidate
        // Add member manually to test non-leader invite
        let party = mgr.parties.get_mut(&id).unwrap();
        party.members.push(PartyMember::new(200));
        mgr.serial_to_party.insert(200, id);

        assert_eq!(mgr.invite(200, 300), Err(PartyError::NotLeader));
    }

    #[test]
    fn invite_fails_when_full() {
        let mut mgr = PartyManager::new();
        let id = mgr.create_party(1).unwrap();
        for i in 2..=MAX_PARTY_SIZE as u32 {
            let party = mgr.parties.get_mut(&id).unwrap();
            party.members.push(PartyMember::new(i));
            mgr.serial_to_party.insert(i, id);
        }
        assert_eq!(
            mgr.invite(1, (MAX_PARTY_SIZE as u32) + 1),
            Err(PartyError::PartyFull)
        );
    }

    #[test]
    fn invite_fails_if_already_candidate() {
        let mut mgr = PartyManager::new();
        mgr.invite(100, 200).unwrap();
        assert_eq!(mgr.invite(100, 200), Err(PartyError::AlreadyCandidate));
    }

    // -- PartyManager: accept_invite -----------------------------------------

    #[test]
    fn accept_invite_success() {
        let mut mgr = PartyManager::new();
        let id = mgr.invite(100, 200).unwrap();
        mgr.accept_invite(200, id).unwrap();
        let party = mgr.parties.get(&id).unwrap();
        assert!(party.has_member(200));
        assert!(!party.has_candidate(200));
        assert_eq!(mgr.serial_to_party.get(&200), Some(&id));
    }

    #[test]
    fn accept_invite_not_candidate() {
        let mut mgr = PartyManager::new();
        let id = mgr.create_party(100).unwrap();
        assert_eq!(
            mgr.accept_invite(999, id),
            Err(PartyError::NotCandidate)
        );
    }

    // -- PartyManager: decline_invite ----------------------------------------

    #[test]
    fn decline_invite_success() {
        let mut mgr = PartyManager::new();
        let id = mgr.invite(100, 200).unwrap();
        mgr.decline_invite(200, id).unwrap();
        let party = mgr.parties.get(&id).unwrap();
        assert!(!party.has_candidate(200));
    }

    #[test]
    fn decline_invite_not_candidate() {
        let mut mgr = PartyManager::new();
        let id = mgr.create_party(100).unwrap();
        assert_eq!(
            mgr.decline_invite(999, id),
            Err(PartyError::NotCandidate)
        );
    }

    // -- PartyManager: remove_member -----------------------------------------

    #[test]
    fn remove_member_self() {
        let mut mgr = PartyManager::new();
        let id = mgr.invite(100, 200).unwrap();
        mgr.accept_invite(200, id).unwrap();
        mgr.remove_member(200, 200).unwrap();
        let party = mgr.parties.get(&id).unwrap();
        assert!(!party.has_member(200));
        assert!(mgr.serial_to_party.get(&200).is_none());
    }

    #[test]
    fn remove_member_leader_kicks() {
        let mut mgr = PartyManager::new();
        let id = mgr.invite(100, 200).unwrap();
        mgr.accept_invite(200, id).unwrap();
        mgr.remove_member(100, 200).unwrap();
        let party = mgr.parties.get(&id).unwrap();
        assert!(!party.has_member(200));
    }

    #[test]
    fn remove_member_non_leader_fails() {
        let mut mgr = PartyManager::new();
        let id = mgr.invite(100, 200).unwrap();
        mgr.accept_invite(200, id).unwrap();
        mgr.invite(100, 300).unwrap();
        mgr.accept_invite(300, id).unwrap();
        assert_eq!(
            mgr.remove_member(200, 300),
            Err(PartyError::NotLeader)
        );
    }

    #[test]
    fn remove_leader_transfers_leadership() {
        let mut mgr = PartyManager::new();
        let id = mgr.invite(100, 200).unwrap();
        mgr.accept_invite(200, id).unwrap();
        mgr.remove_member(100, 100).unwrap();
        let party = mgr.parties.get(&id).unwrap();
        assert_eq!(party.leader, 200);
    }

    #[test]
    fn remove_last_member_disbands() {
        let mut mgr = PartyManager::new();
        let id = mgr.create_party(100).unwrap();
        mgr.remove_member(100, 100).unwrap();
        assert!(mgr.parties.get(&id).is_none());
        assert!(mgr.serial_to_party.get(&100).is_none());
    }

    // -- PartyManager: set_leader --------------------------------------------

    #[test]
    fn set_leader_success() {
        let mut mgr = PartyManager::new();
        let id = mgr.invite(100, 200).unwrap();
        mgr.accept_invite(200, id).unwrap();
        mgr.set_leader(100, 200).unwrap();
        let party = mgr.parties.get(&id).unwrap();
        assert_eq!(party.leader, 200);
    }

    #[test]
    fn set_leader_not_leader() {
        let mut mgr = PartyManager::new();
        let id = mgr.invite(100, 200).unwrap();
        mgr.accept_invite(200, id).unwrap();
        assert_eq!(mgr.set_leader(200, 100), Err(PartyError::NotLeader));
    }

    #[test]
    fn set_leader_target_not_in_party() {
        let mut mgr = PartyManager::new();
        mgr.create_party(100).unwrap();
        assert_eq!(mgr.set_leader(100, 999), Err(PartyError::NotInParty));
    }

    // -- PartyManager: disband -----------------------------------------------

    #[test]
    fn disband_success() {
        let mut mgr = PartyManager::new();
        let id = mgr.invite(100, 200).unwrap();
        mgr.accept_invite(200, id).unwrap();
        let members = mgr.disband(100).unwrap();
        assert_eq!(members, vec![100, 200]);
        assert!(mgr.parties.is_empty());
        assert!(mgr.serial_to_party.is_empty());
    }

    #[test]
    fn disband_not_leader() {
        let mut mgr = PartyManager::new();
        let id = mgr.invite(100, 200).unwrap();
        mgr.accept_invite(200, id).unwrap();
        assert_eq!(mgr.disband(200), Err(PartyError::NotLeader));
    }

    // -- Packet tests --------------------------------------------------------

    #[test]
    fn packet_add_member_structure() {
        let pkt = packets::party_add_member(&[0x00000001, 0x00000002]);
        assert_eq!(pkt[0], 0xBF); // packet id
        // length = 1 (id) + 2 (len) + 2 (subcmd) + 1 (sub-sub) + 1 (count) + 8 (2 serials) = 15
        assert_eq!(pkt[1], 0x00);
        assert_eq!(pkt[2], 15);
        // sub-command 0x0006
        assert_eq!(pkt[3], 0x00);
        assert_eq!(pkt[4], 0x06);
        // sub-sub command 0x01
        assert_eq!(pkt[5], 0x01);
        // member count
        assert_eq!(pkt[6], 2);
        // first serial
        assert_eq!(&pkt[7..11], &[0x00, 0x00, 0x00, 0x01]);
        // second serial
        assert_eq!(&pkt[11..15], &[0x00, 0x00, 0x00, 0x02]);
    }

    #[test]
    fn packet_remove_member_structure() {
        let pkt = packets::party_remove_member(0x00000003, &[0x00000001, 0x00000002]);
        assert_eq!(pkt[0], 0xBF);
        // length = 1 + 2 + 2 + 1 + 1 + 4 + 8 = 19
        assert_eq!(pkt[1], 0x00);
        assert_eq!(pkt[2], 19);
        assert_eq!(pkt[5], 0x02); // sub-sub command: remove
        assert_eq!(pkt[6], 2);    // remaining count
        // removed serial
        assert_eq!(&pkt[7..11], &[0x00, 0x00, 0x00, 0x03]);
        // remaining serials
        assert_eq!(&pkt[11..15], &[0x00, 0x00, 0x00, 0x01]);
        assert_eq!(&pkt[15..19], &[0x00, 0x00, 0x00, 0x02]);
    }

    #[test]
    fn packet_invite_structure() {
        let pkt = packets::party_invite(0xDEADBEEF);
        assert_eq!(pkt[0], 0xBF);
        // length = 1 + 2 + 2 + 1 + 4 = 10
        assert_eq!(pkt[1], 0x00);
        assert_eq!(pkt[2], 10);
        assert_eq!(pkt[5], 0x07); // sub-sub command: invite
        assert_eq!(&pkt[6..10], &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn packet_accept_structure() {
        let pkt = packets::party_accept(0x12345678);
        assert_eq!(pkt[0], 0xBF);
        assert_eq!(pkt[5], 0x09); // sub-sub command: accept
        assert_eq!(&pkt[6..10], &[0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn packet_decline_structure() {
        let pkt = packets::party_decline(0xCAFEBABE);
        assert_eq!(pkt[0], 0xBF);
        assert_eq!(pkt[5], 0x08); // sub-sub command: decline
        assert_eq!(&pkt[6..10], &[0xCA, 0xFE, 0xBA, 0xBE]);
    }
}
