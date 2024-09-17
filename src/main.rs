use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};

const LOCKED: bool = true;
const UNLOCKED: bool = false;

pub struct Mutex<T> {
    locked: AtomicBool,
    v: UnsafeCell<T>,
}

unsafe impl<T> Sync for Mutex<T> where T: Send {}

impl<T> Mutex<T> {
    pub fn new(t: T) -> Self {
        Self {
            locked: AtomicBool::new(UNLOCKED),
            v: UnsafeCell::new(t),
        }
    }

    pub fn with_lock<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        while self
            .locked
            .compare_exchange_weak(UNLOCKED, LOCKED, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            std::thread::yield_now();
        }
        let ret = f(unsafe { &mut *self.v.get() });
        self.locked.store(UNLOCKED, Ordering::Release);
        ret
    }
}

use std::thread::spawn;
fn main() {
    let l: &'static _ = Box::leak(Box::new(Mutex::new(0)));
    let handles: Vec<_> = (0..10)
        .map(|_| {
            spawn(move || {
                for _ in 0..100 {
                    l.with_lock(|v| {
                        *v += 1;
                    })
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(l.with_lock(|v| *v), 1000);
}

#[test]
fn relaxed() {
    use std::sync::atomic::AtomicUsize;

    let x: &'static _ = Box::leak(Box::new(AtomicUsize::new(1)));
    let y: &'static _ = Box::leak(Box::new(AtomicUsize::new(1)));

    let t1 = spawn(|| {
        let r1 = y.load(Ordering::Relaxed);
        x.store(r1, Ordering::Relaxed);
        r1
    });

    let t2 = spawn(|| {
        let r2 = x.load(Ordering::Relaxed);
        y.store(42, Ordering::Relaxed);
        r2
    });

    let r1 = t1.join().unwrap();
    let r2 = t2.join().unwrap();

    // r1 == r2 == 42
}
