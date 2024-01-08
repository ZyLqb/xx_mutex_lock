use core::sync::atomic::{AtomicU8, Ordering};
///一共四种状态
/// 用来表示once的运行状态
///
pub mod status {
    pub const INCOMPLETE: u8 = 0x00;
    pub const RUNNING: u8 = 0x01;
    pub const COMPLETE: u8 = 0x02;
    pub const PANICKED: u8 = 0x03;
}
use status::*;

/// 确保一段代码即使是在多线程的情况下，也只执行一次
/// # Example
/// ```
/// use crate::once::Once;
/// let once = Once::new();
/// once.call_once(|| {
///     //run some code here
/// });
/// ```
pub(crate) struct Once {
    status: AtomicU8,
}

unsafe impl Sync for Once {}
unsafe impl Send for Once {}
impl Once {
    pub const fn new() -> Self {
        Self {
            status: AtomicU8::new(INCOMPLETE),
        }
    }

    #[inline]
    pub fn is_completed(&self) -> bool {
        self.status.load(Ordering::Acquire) == COMPLETE
    }
    ///
    /// 运行只运行一次的代码
    ///
    /// # Example
    /// ```
    /// use crate::once::Once;
    /// let once = Once::new();
    /// once.call_once(|| {
    ///     std::println!("I only run once")
    /// });
    /// ```
    #[inline]
    pub fn call_once<F: FnOnce()>(&self, f: F) {
        //如果没有被初始化才调用
        if !self.is_completed() {
            self.call(f);
        }
    }

    #[cold]
    fn call<F: FnOnce()>(&self, f: F) {
        loop {
            // compare_exchange 是原子的交换两个数字，他的返回值是Result
            // 只有一个线程会成功，成功后将status的值设置成RUNNING
            // 如果失败了会返回当前status里面的值
            let xchg = self.status.compare_exchange(
                status::INCOMPLETE,
                status::RUNNING,
                Ordering::Acquire,
                Ordering::Acquire,
            );
            match xchg {
                Ok(_must_be_incomplete) => {
                    //为了易读性，实现写在下面
                }
                //另一个线程持有了锁，并且panic了
                Err(status::PANICKED) => panic!("Once paniced"),
                //另一个线程正在运行
                Err(status::RUNNING) => match self.poll() {
                    Ok(_) => return,
                    Err(_) => continue,
                },
                //另一个线程完成了
                Err(status::COMPLETE) => return,
                //因为其他原因没交换成功（不应该出现这种情况）
                Err(status::INCOMPLETE) => continue,
                Err(_) => panic!("never run here"),
            }
            //这个finish用于判断是否在持有锁的时候panic掉了
            //在panic时会drop所持有所有权的数据，
            //在这个finish drop的时候，将状态设置为panic,让其他线程知道有个线程panic了
            let finish = Finish {
                status: &self.status,
            };
            //运行所要运行的代码
            f();
            //正常结束，forget掉这个finish,不要设置status为panic
            core::mem::forget(finish);
            //将状态设置为complete
            self.status.store(status::COMPLETE, Ordering::Release);
            return;
        }
    }

    fn poll(&self) -> Result<(), u8> {
        loop {
            match self.status.load(Ordering::Acquire) {
                status::INCOMPLETE => return Err(INCOMPLETE),
                status::RUNNING => core::hint::spin_loop(),
                status::COMPLETE => return Ok(()),
                status::PANICKED => panic!("Once previously poisoned by a panicked"),
                _ => {
                    panic!("never run here")
                }
            }
        }
    }
}

pub struct Finish<'a> {
    status: &'a AtomicU8,
}

impl<'a> Drop for Finish<'a> {
    fn drop(&mut self) {
        self.status.store(status::PANICKED, Ordering::SeqCst)
    }
}

#[cfg(test)]
pub mod test {
    extern crate std;
    use crate::once::Once;
    use std::println;
    #[test]
    fn test() {
        let once = Once::new();
        let arc_once = std::sync::Arc::new(once);
        let once_1 = arc_once.clone();
        let once_2 = arc_once.clone();
        let t1 = std::thread::spawn(move || {
            once_1.call_once(|| println!("hello , I only hello once"));
        });

        let t2 = std::thread::spawn(move || {
            once_2.call_once(|| println!("hello , I only hello once"));
        });

        t1.join().expect("Err");
        t2.join().expect("Err");
    }
}
