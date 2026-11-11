//! DIS receiver thread for the GUI: owns the UDP socket, applies PDUs to a
//! shared entity table, and wakes the UI when something changes.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::dis::receiver::{Receiver, Stats};
use crate::dis::{DisConfig, EntityTable, ENTITY_TIMEOUT_SECS};

#[derive(Default)]
pub struct Shared {
    pub table: EntityTable,
    pub stats: Stats,
    /// Set if the socket could not be opened; the thread then exits.
    pub error: Option<String>,
}

/// Start the receiver thread. `wake` is called after each change so the UI
/// can repaint (egui's `Context::request_repaint`).
pub fn spawn(cfg: DisConfig, wake: impl Fn() + Send + 'static) -> Arc<Mutex<Shared>> {
    let shared = Arc::new(Mutex::new(Shared::default()));
    let out = Arc::clone(&shared);
    thread::Builder::new()
        .name("dis-receiver".into())
        .spawn(move || run(cfg, out, wake))
        .expect("spawn DIS receiver thread");
    shared
}

fn run(cfg: DisConfig, shared: Arc<Mutex<Shared>>, wake: impl Fn()) {
    let mut receiver = match Receiver::bind(&cfg) {
        Ok(r) => r,
        Err(e) => {
            shared.lock().unwrap().error = Some(format!("cannot listen on UDP {}: {e}", cfg.port));
            wake();
            return;
        }
    };
    let mut last_purge = Instant::now();
    loop {
        let pdus = match receiver.poll(Duration::from_millis(50)) {
            Ok(p) => p,
            Err(e) => {
                shared.lock().unwrap().error = Some(format!("receive error: {e}"));
                thread::sleep(Duration::from_millis(200));
                continue;
            }
        };
        let now = Instant::now();
        let purge = now.duration_since(last_purge) >= Duration::from_secs(1);
        if pdus.is_empty() && !purge {
            continue;
        }
        {
            let mut s = shared.lock().unwrap();
            for pdu in &pdus {
                s.table.apply(pdu, now);
            }
            if purge {
                s.table.purge_stale(ENTITY_TIMEOUT_SECS);
                last_purge = now;
            }
            s.stats = receiver.stats;
            s.error = None;
        }
        wake();
    }
}
