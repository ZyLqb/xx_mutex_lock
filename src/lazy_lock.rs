use core::{cell::Cell, ops::Deref};

use super::once_lock::OnceLock;

///
/// 将初始化推迟到第一次访问的时候再初始化
/// # Example
/// ```
///  let lazy = LazyLock::new(|| 1 + 3);
///  let a = **lazy;
///  std::println!("I am {}", a);
/// ```
pub struct LazyLock<T, F = fn() -> T> {
    cell: OnceLock<T>,
    init: Cell<Option<F>>,
}

unsafe impl<T, F: Send> Sync for LazyLock<T, F> where OnceLock<T>: Sync {}

impl<T, F> LazyLock<T, F> {
    pub const fn new(f: F) -> Self {
        Self {
            cell: OnceLock::new(),
            init: Cell::new(Some(f)),
        }
    }
    ///
    /// 在解引用的时候调用force初始化
    /// 这样就可以达到在访问的时候初始化
    ///
    fn force(this: &Self) -> &T
    where
        F: FnOnce() -> T,
    {
        this.cell.get_or_init(|| match this.init.take() {
            Some(f) => f(),
            None => panic!("Lazy instance has previously been poisoned"),
        })
    }

    pub fn get(&self) -> Option<&T> {
        self.cell.get()
    }
}
///在解引用的时候调用force初始化
impl<T, F: FnOnce() -> T> Deref for LazyLock<T, F> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        Self::force(self)
    }
}

impl<T: Default> Default for LazyLock<T, fn() -> T> {
    fn default() -> Self {
        Self::new(T::default)
    }
}

#[cfg(test)]
pub mod test {
    extern crate std;

    use crate::lazy_lock::LazyLock;
    #[test]
    fn test() {
        let lazy = LazyLock::new(|| 1 + 3);
        let arc_once = std::sync::Arc::new(lazy);
        let once_1 = arc_once.clone();
        let once_2 = arc_once.clone();
        let t1 = std::thread::spawn(move || {
            let a = **once_1;
            std::println!("i am first t1? {}", a);
        });

        let t2 = std::thread::spawn(move || {
            let a = **once_2;
            std::println!("Im first t2 ?  {}", a);
        });

        t1.join().expect("Err");
        t2.join().expect("Err");

        let c = **arc_once;
        std::println!("{}", c);
    }
}
