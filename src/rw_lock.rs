use core::{
    cell::UnsafeCell,
    //ptr::NonNull,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicIsize, Ordering},
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
pub struct RWLock<T> {
    pub(crate) lock: AtomicIsize,
    data: UnsafeCell<T>,
}

/// 使用iszie保存锁的状态：
/// 正数表示读锁，同时可以作为读锁的计数
/// -1 表示写锁，只有一种状态
const READED: isize = 1;
const WRITED: isize = -1;

/// 读锁守卫
pub struct RWLockReadGuard<'a, T> {
    inner: &'a RWLock<T>,
    data: *const T,
}

/// 写锁守卫
pub struct RWLockWriteGuard<'a, T> {
    inner: &'a RWLock<T>,
    data: *mut T,
}

unsafe impl<T: Send> Send for RWLock<T> {}
unsafe impl<T: Send + Sync> Sync for RWLock<T> {}

impl<T> RWLock<T> {
    pub const fn new(data: T) -> Self {
        RWLock {
            lock: AtomicIsize::new(0),
            data: UnsafeCell::new(data),
        }
    }

    /// 获取写锁
    #[inline]
    pub fn write(&self) -> RWLockWriteGuard<T> {
        loop {
            match self.try_write() {
                Some(guard) => return guard,
                None => continue,
            }
        }
    }

    /// 非阻塞地获取写锁
    #[inline]
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

    #[inline]
    fn write_request(&self) -> bool {
        if self
            .lock
            .compare_exchange(0, WRITED, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            true
        } else {
            false
        }
    }

    /// 获取读锁
    #[inline]
    pub fn read(&self) -> RWLockReadGuard<T> {
        loop {
            match self.try_read() {
                Some(guard) => return guard,
                None => continue,
            }
        }
    }

    /// 非阻塞地获取读锁
    #[inline]
    pub fn try_read(&self) -> Option<RWLockReadGuard<T>> {
        if self.read_request() >= 0 {
            Some(RWLockReadGuard {
                inner: &self,
                data: self.data.get(),
            })
        } else {
            None
        }
    }

    #[inline]
    fn read_request(&self) -> isize {
        const MAX_READERS: isize = core::isize::MAX;
        let mut readers = self.lock.load(Ordering::Acquire);

        if readers >= MAX_READERS && readers < 0 {
            // panic!("read request wrong");
            -1
        } else {
            readers = self.lock.fetch_add(READED, Ordering::Relaxed);
            readers
        }
    }
}

impl<'a, T> Deref for RWLockReadGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.data }
    }
}

impl<'a, T> Deref for RWLockWriteGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.data }
    }
}

impl<'a, T> Drop for RWLockReadGuard<'a, T> {
    fn drop(&mut self) {
        self.inner.lock.fetch_sub(READED, Ordering::Release);
    }
}

impl<'a, T> Drop for RWLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        self.inner.lock.fetch_sub(WRITED, Ordering::Release);
    }
}

impl<'a, T> DerefMut for RWLockWriteGuard<'a, T> {
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
        let m = RWLock::new(0);
        let wlock = m.try_write();

        assert_eq!(-1, m.read_request());
        drop(wlock);

        let mut i = 0;
        while i < 100 {
            i += 1;
            assert_eq!(i, m.read_request());
        }

        assert!(!m.write_request());
    }

    #[test]
    fn test_rw_try_read() {
        let m = RWLock::new(0);
        let wlock = m.try_write();

        assert!(m.try_read().is_none());
        drop(wlock);

        let mut i = 0;
        while i < 100 {
            assert!(m.try_read().is_some());
            i += 1;
        }

        assert!(m.try_write().is_none());
    }

    #[test]
    fn test() {
        let data = RWLock::new(0);
        let read_lock1 = data.read();
        let read_lock2 = data.read();

        assert_eq!(0, *read_lock1);
        assert_eq!(0, *read_lock2);

        assert!(data.try_write().is_none());

        drop(read_lock1);
        drop(read_lock2);

        let mut write_lock = data.write();
        *write_lock += 1;

        assert!(data.try_write().is_none());
        assert!(data.try_read().is_none());

        assert_eq!(1, *write_lock);
    }
}
