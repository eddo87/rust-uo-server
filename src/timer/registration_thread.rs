use super::Timer;
use crate::error::ServerError;
use log::debug;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

pub fn spawn(register_rx: mpsc::Receiver<Timer>, new_timers: Arc<Mutex<Vec<Timer>>>) {
    thread::spawn(move || {
        debug!("Timer registration thread started");
        for timer in register_rx {
            debug!("Registering new timer: interval={}ms, repetitions={}", timer.interval, timer.repetitions);
            let mut new_timers = match new_timers.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    eprintln!("{}", ServerError::TimerLockPoisoned);
                    eprintln!("Poison error details: {}", e);
                    return;
                }
            };
            new_timers.push(timer);
        }
    });
}
