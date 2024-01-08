#![no_std]
#![feature(never_type)]
#![feature(dropck_eyepatch)]
pub mod lazy_lock;
pub mod mutex;
pub mod once;
pub mod once_lock;
pub mod rw_lock;

pub use lazy_lock::LazyLock;
pub use mutex::Mutex;
pub use mutex::MutexGuard;
pub use once_lock::OnceLock;
pub use rw_lock::RWLock;
pub use rw_lock::RWLockReadGuard;
pub use rw_lock::RWLockWriteGuard;
