use core::{
    cell::UnsafeCell,
    //ptr::NonNull,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicUsize, Ordering},
};

/// 读写锁
/// 读写操作分离，分为了读锁和写锁，写锁将限制了仅一
/// 个线程的临界区进行读操作，而读锁允许多个线程的临
/// 界区进写操作
/// #Example:
/// ```
/// use xx_mutex_lock::rw_lock::RWLock;
///
/// {
///     let data = RWLock::new(0);
///     let read_lock1 = data.read();
///     let read_lock2 = data.read();
///     println!("{}", *read_lock1);
///     println!("{}", *read_lock2);
///
///     drop(read_lock1);
///     drop(read_lock2);
///
///     let mut write_lock = data.write();
///     *write_lock += 1;
/// } // 这里drop
/// ```
pub struct RWLock<T: ?Sized> {
    pub(crate) lock: AtomicUsize,
    data: UnsafeCell<T>,
}

/// 使用usize的最后一位保存WRITED，剩下的用来保存
/// READED(也就是一个WRITED和(usize/2)个READED)
const READED: usize = 1 << 1;
const WRITED: usize = 1;

/// 读锁守卫
pub struct RWLockReadGuard<'a, T: ?Sized> {
    inner: &'a RWLock<T>,
    data: *const T,
}

/// 写锁守卫
pub struct RWLockWriteGuard<'a, T: ?Sized + 'a> {
    inner: &'a RWLock<T>,
    data: *mut T,
}

unsafe impl<T: ?Sized + Send> Send for RWLock<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for RWLock<T> {}

impl<T> RWLock<T> {
    pub fn new(data: T) -> Self {
        RWLock {
            lock: AtomicUsize::new(0),
            data: UnsafeCell::new(data),
        }
    }

    /// 获取写锁
    pub fn write(&self) -> RWLockWriteGuard<T> {
        loop {
            match self.try_write() {
                Some(guard) => return guard,
                None => continue,
            }
        }
    }

    /// 非阻塞地获取写锁
    pub fn try_write(&self) -> Option<RWLockWriteGuard<T>> {
        if self.write_request() {
            Some(RWLockWriteGuard {
                inner: &self,
                data: self.data.get(),
            })
        } else {
            None
        }
    }

    fn write_request(&self) -> bool {
        if self
            .lock
            .compare_exchange(0, WRITED, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            self.lock.fetch_add(WRITED, Ordering::Acquire);
            true
        } else {
            false
        }
    }

    fn read_request(&self) -> usize {
        const MAX_READERS: usize = core::usize::MAX / READED / 2;
        let prev_readers = self.lock.fetch_add(READED, Ordering::Acquire);

        if prev_readers >= MAX_READERS * READED {
            self.lock.fetch_sub(READED, Ordering::Relaxed);
            panic!("too many readers");
        } else {
            prev_readers
        }
    }

    /// 获取读锁
    pub fn read(&self) -> RWLockReadGuard<T> {
        loop {
            match self.try_read() {
                Some(guard) => return guard,
                None => continue,
            }
        }
    }

    /// 非阻塞地获取读锁
    pub fn try_read(&self) -> Option<RWLockReadGuard<T>> {
        if (self.read_request() | WRITED) != 0 {
            Some(RWLockReadGuard {
                inner: &self,
                data: self.data.get(),
            })
        } else {
            None
        }
    }
}

impl<'a, T: ?Sized> Deref for RWLockReadGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.data }
    }
}

impl<'a, T: ?Sized> Deref for RWLockWriteGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.data }
    }
}

impl<'a, T: ?Sized> Drop for RWLockReadGuard<'a, T> {
    fn drop(&mut self) {
        self.inner.lock.fetch_sub(READED, Ordering::Release);
    }
}

impl<'a, T: ?Sized> Drop for RWLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        self.inner.lock.load(Ordering::Relaxed);
    }
}

impl<'a, T: ?Sized> DerefMut for RWLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.data }
    }
}

#[cfg(test)]
pub mod test {
    extern crate std;

    use crate::rw_lock::RWLock;
    //use std::println;

    #[test]
    fn test_rw_write_request() {
        let m = RWLock::new(0);
        m.write_request();

        assert!(!m.write_request());
    }

    #[test]
    fn test_rw_try_write() {
        let m = RWLock::new(0);
        m.write_request();

        assert!(m.try_write().is_none());
    }

    #[test]
    fn test_rw_read_request() {
        const READED: usize = 1 << 1;
        let m = RWLock::new(0);

        let mut i = 0;
        while i < 100 {
            let value = m.read_request();
            assert_eq!(READED * i, value);
            i += 1;
        }
    }

    #[test]
    fn test_rw_try_read() {
        let m = RWLock::new(0);

        let mut i = 0;
        while i < 100 {
            assert!(m.try_read().is_some());
            i += 1;
        }
    }

    #[test]
    fn test() {
        let data = RWLock::new(0);
        let read_lock1 = data.read();
        let read_lock2 = data.read();

        assert_eq!(0, *read_lock1);
        assert_eq!(0, *read_lock2);

        drop(read_lock1);
        drop(read_lock2);

        let mut write_lock = data.write();
        *write_lock += 1;

        assert_eq!(1, *write_lock);
    }
}
