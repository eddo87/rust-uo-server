use log::debug;

pub struct StateDelta {
    pub property: String,
    pub delta: i32,
}

pub trait State {
    fn update_state(&mut self, state_deltas: &Vec<StateDelta>);
}

pub struct Character {
    pub hitpoints: i32,
    pub name: String,
}

impl State for Character {
    fn update_state(&mut self, state_deltas: &Vec<StateDelta>) {
        for state_delta in state_deltas {
            if state_delta.property == "hitpoints" {
                self.hitpoints += state_delta.delta;
                debug!("Character {} hitpoints are now: {}", self.name, self.hitpoints);
            }
        }
    }
}

pub struct Monster {
    pub anger: i32,
    pub name: String,
}

impl State for Monster {
    fn update_state(&mut self, state_deltas: &Vec<StateDelta>) {
        for state_delta in state_deltas {
            if state_delta.property == "anger" {
                self.anger += state_delta.delta;
                debug!("Monster {} anger is now: {}", self.name, self.anger);
            }
        }
    }
}

pub struct Shard {
    name: String,
    percent_full: u8,
    timezone: u8,
    address: [u8; 4],
}

impl Shard {
    pub fn new(name: String) -> Shard {
        Shard { name, percent_full: 0, timezone: 0x00, address: [127, 0, 0, 1] }
    }
}
