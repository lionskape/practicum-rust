use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn race_increment(iterations: usize, threads: usize) -> u64 {
    COUNTER.store(0, Ordering::SeqCst);
    let mut handles = Vec::with_capacity(threads);
    for _ in 0..threads {
        handles.push(thread::spawn(move || {
            for _ in 0..iterations {
                COUNTER.fetch_add(1, Ordering::SeqCst);
            }
        }));
    }
    for h in handles {
        let _ = h.join();
    }
    COUNTER.load(Ordering::SeqCst)
}

pub fn read_after_sleep() -> u64 {
    COUNTER.load(Ordering::SeqCst)
}
