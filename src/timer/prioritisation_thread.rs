use super::Timer;
use crate::error::ServerError;
use crate::ticks::current_ticks;
use log::{debug, trace};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

pub fn spawn(execute_tx: mpsc::Sender<Timer>, new_timers: Arc<Mutex<Vec<Timer>>>) {
    thread::spawn(move || {
        debug!("Timer prioritisation thread started");
        let mut timers: Vec<Timer> = Vec::new();
        loop {
            thread::sleep(Duration::from_millis(1));
            {
                let mut new_timers = match new_timers.lock() {
                    Ok(guard) => guard,
                    Err(e) => {
                        eprintln!("{}", ServerError::TimerLockPoisoned);
                        eprintln!("Poison error details: {}", e);
                        return;
                    }
                };
                while let Some(timer) = new_timers.pop() {
                    trace!("Moving new timer into active list");
                    timers.push(timer);
                }
            }
            let mut not_due = vec![];
            let now = current_ticks();
            for timer in timers {
                if timer.next > now {
                    not_due.push(timer);
                } else {
                    trace!("Timer due, sending for execution");
                    if let Err(e) = execute_tx.send(timer) {
                        eprintln!("Failed to send timer for execution: {}", e);
                        return;
                    }
                }
            }
            timers = not_due;
        }
    });
}
