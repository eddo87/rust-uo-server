use super::Timer;
use log::{debug, trace};
use std::sync::mpsc;
use std::thread;

pub fn spawn(execute_rx: mpsc::Receiver<Timer>, register_tx: mpsc::Sender<Timer>) {
    thread::spawn(move || {
        debug!("Timer execution thread started");
        for mut timer in execute_rx {
            trace!("Executing timer callback, repetitions remaining: {}", timer.repetitions);
            (timer.callback)();
            timer.repetitions -= 1;
            if timer.repetitions > 0 {
                timer.next = timer.next + timer.interval;
                trace!("Re-registering timer, next execution at tick {}", timer.next);
                if let Err(e) = register_tx.send(timer) {
                    eprintln!("Failed to re-register timer: {}", e);
                    return;
                }
            } else {
                debug!("Timer completed all repetitions");
            }
        }
    });
}
