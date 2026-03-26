use crate::error::ServerError;
use crate::state::{Character, Monster};
use crate::ticks::current_ticks;
use crate::timer::Timer;
use log::debug;
use std::sync::mpsc;

pub fn start(timer_register_tx: mpsc::Sender<Timer>) -> Result<(), ServerError> {
    let repetitions = 2;
    let interval = 1000;
    let next = current_ticks() + interval;
    let mut state = Character { name: String::from("Bob"), hitpoints: 100 };
    let callback = Box::new(move || {
        state.hitpoints -= 1;
        debug!("Character {} hitpoints are now: {}", state.name, state.hitpoints);
    });
    let timer = Timer { repetitions, interval, next, callback };
    timer_register_tx.send(timer).map_err(|_| ServerError::TimerSend)?;

    let repetitions = 2;
    let interval = 500;
    let next = current_ticks() + interval;
    let mut state = Monster { name: String::from("Dave"), anger: 0 };
    let callback = Box::new(move || {
        state.anger += 10;
        debug!("Monster {} anger is now: {}", state.name, state.anger);
    });
    let timer = Timer { repetitions, interval, next, callback };
    timer_register_tx.send(timer).map_err(|_| ServerError::TimerSend)?;
    Ok(())
}
