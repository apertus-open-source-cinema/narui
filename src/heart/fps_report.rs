use std::{
    sync::mpsc::{channel, Sender},
    thread,
    time::{Duration, SystemTime},
};

pub struct FPSReporter {
    tx: Sender<()>,
}

impl FPSReporter {
    pub fn new(name: &str) -> Self {
        let (tx, rx) = channel();
        let name = name.to_string();
        thread::spawn(move || {
            let mut time = SystemTime::now();
            let mut frames = 0u128;
            loop {
                if rx.recv_timeout(Duration::from_millis(100)).is_ok() {
                    frames += 1;
                }
                let current_time = SystemTime::now();
                let elapsed_ms = current_time.duration_since(time).unwrap().as_millis();
                if elapsed_ms > 1000 {
                    println!("{}: {}fps", name, (frames as f64 / elapsed_ms as f64 * 1000f64));
                    time = current_time;
                    frames = 0;
                }
            }
        });

        Self { tx }
    }

    pub fn frame(&mut self) { self.tx.send(()).unwrap(); }
}
