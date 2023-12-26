use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

pub struct Mutex<T: ?Sized> {
    pub(crate) lock: AtomicBool,
    data: UnsafeCell<T>,
}

pub struct MutexGuard<'a, T: ?Sized + 'a> {
    lock: &'a AtomicBool,
    data: *mut T,
}

unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}

unsafe impl<T: ?Sized + Sync> Sync for MutexGuard<'_, T> {}
unsafe impl<T: ?Sized + Send> Send for MutexGuard<'_, T> {}

impl<T> Mutex<T> {
    pub const fn new(data: T) -> Self {
        Mutex {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn is_locked(&self) -> bool {
        self.lock.load(Ordering::Relaxed)
    }

    pub fn lock(&self) -> MutexGuard<T> {
        while self
            .lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            while self.is_locked() {
                core::hint::spin_loop();
            }
        }
        MutexGuard {
            lock: &self.lock,
            data: self.data.get(),
        }
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.data }
    }
}

impl<'a, T: ?Sized> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release)
    }
}
#[cfg(test)]
pub mod test {
    extern crate std;
    use crate::mutex::Mutex;
    #[test]
    fn test() {
        use std::sync::Arc;
        let lock = Mutex::new(1);
        let lock = Arc::new(lock);
        let t1_lock = lock.clone();
        let t2_lock = lock.clone();
        let t1 = std::thread::spawn(move || {
            for _ in 0..100 {
                let mut locked = t1_lock.lock();
                *locked += 1;
            }
        });

        let t2 = std::thread::spawn(move || {
            for _ in 0..100 {
                let mut locked = t2_lock.lock();
                *locked += 1;
            }
        });
        t1.join().expect("err");
        t2.join().expect("err");
        let c = lock.lock();
        assert_eq!(*c,201)
    }
}
