#![no_std]
#![feature(never_type)]
#![feature(dropck_eyepatch)]
pub mod lazy_lock;
pub mod mutex;
pub mod once;
pub mod once_lock;

pub use mutex::Mutex;
pub use mutex::MutexGuard;
pub use lazy_lock::LazyLock;
pub use once_lock::OnceLock;
