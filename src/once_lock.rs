use core::{cell::UnsafeCell, marker::PhantomData, mem::MaybeUninit};

use super::once::Once;

/// 用于初始化全局变量，只能初始化一次，不能改变
/// # Example
/// 
/// ```
/// use crate::once_lock::OnceLock;
/// 
/// let INIT = OnceLock::new();
/// 
/// INIT.get_or_init({
///     //run init code here
/// })
/// ```
pub struct OnceLock<T = ()> {
    once: Once,
    data: UnsafeCell<MaybeUninit<T>>,
    _marker: PhantomData<T>,
}

unsafe impl<T: Sync + Send> Sync for OnceLock<T> {}
unsafe impl<T: Send> Send for OnceLock<T> {}

impl<T> OnceLock<T> {
    pub const fn new() -> Self {
        Self {
            once: Once::new(),
            data: UnsafeCell::new(MaybeUninit::uninit()),
            _marker: PhantomData,
        }
    }
    #[inline]
    fn is_initialized(&self) -> bool {
        self.once.is_completed()
    }

    ///用法
    /// ```
    ///use crate::once_lock::OnceLock;
    /// let INIT = OnceLock::new(); 
    /// 
    /// INIT.get_or_init(|| 3);
    ///  
    /// assert_eq!(3,INIT.get())
    /// ```
    #[inline]
    pub fn get(&self) -> Option<&T> {
        //只在已经被初始化的情况下返回
        if self.is_initialized() {
            Some(unsafe { self.get_unchecked() })
        } else {
            None
        }
    }
    ///用法
    /// ```
    ///use crate::once_lock::OnceLock;
    /// let INIT = OnceLock::new(); 
    /// 
    /// INIT.set(|| 3);
    ///  
    /// assert_eq!(3,INIT.get())
    /// ```
    #[inline]
    pub fn set(&self, data: T) -> Result<(), (&T, T)> {
        let mut data = Some(data);
        let res = self.get_or_init(|| data.take().unwrap());
        match data {
            None => Ok(()),
            Some(value) => Err((res, value)),
        }
    }
    #[inline]
    pub fn get_mut(&mut self) -> Option<&mut T> {
        if self.is_initialized() {
            Some(unsafe { self.get_unchecked_mut() })
        } else {
            None
        }
    }

    //用于初始化的方法，
    //可以传入一个闭包,具体用法参见上面的例子
    /// ```
    ///use crate::once_lock::OnceLock;
    /// let INIT = OnceLock::new(); 
    /// 
    /// INIT.get_or_init(|| 3);
    ///  
    /// assert_eq!(3,INIT.get())
    /// ```
    #[inline]
    pub fn get_or_init<F: FnOnce() -> T>(&self, f: F) -> &T {
        //将闭包转化为 返回Result的闭包（实际上这里以我的实现只可能返回Ok）
        match self.try_get_or_init(|| Ok::<T, !>(f())) {
            Ok(data) => data,
            Err(_) => panic!("never"),
        }
    }

    #[inline]
    fn try_get_or_init<E, F: FnOnce() -> Result<T, E>>(&self, f: F) -> Result<&T, E> {
        //如果此时已经有被初始化了，直接返回
        if let Some(value) = self.get() {
            return Ok(value);
        }
        //实际的初始化函数
        self.initialized(f)?;

        assert!(self.is_initialized());

        Ok(unsafe { self.get_unchecked() })
    }
    #[cold]
    fn initialized<F: FnOnce() -> Result<T, E>, E>(&self, f: F) -> Result<(), E> {
        let mut res = Ok(());
        let slot = &self.data;
        //调用Once的方法，保持多线程下也只运行一次 f
        self.once.call_once(|| match f() {
            Ok(data) => {
                //如果成功，则将值写入Self的UnsafeCell
                unsafe { (*slot.get()).write(data) };
            }
            Err(e) => {
                res = Err(e);
                panic!("once lock error panic")
            }
        });
        res
    }

    #[inline]
    unsafe fn get_unchecked(&self) -> &T {
        (*self.data.get()).assume_init_ref()
    }
    #[inline]
    unsafe fn get_unchecked_mut(&mut self) -> &mut T {
        (*self.data.get()).assume_init_mut()
    }
    #[inline]
    pub fn as_mut_ptr(&self) -> *mut T {
        self.data.get().cast::<T>()
    }
}



unsafe impl<#[may_dangle] T> Drop for OnceLock<T> {
    fn drop(&mut self) {
        if self.is_initialized() {
            unsafe { (*self.data.get()).assume_init_drop() }
        }
    }
}

#[cfg(test)]
pub mod test {
    extern crate std;
    use crate::once_lock::OnceLock;
    #[test]
    fn test() {
        let once = OnceLock::new();
        let arc_once = std::sync::Arc::new(once);
        let once_1 = arc_once.clone();
        let once_2 = arc_once.clone();
        let t1 = std::thread::spawn(move || {
            std::println!("im first t1");
            let _ = once_1.get_or_init( || 1);
        });

        let t2 = std::thread::spawn(move || {
            std::println!("im first t2");
            let _ = once_2.get_or_init(|| 2);
        });

        t1.join().expect("Err");
        t2.join().expect("Err");

        let c = arc_once.get();
        //这个值等于先运行的线程的初始化的值
        std::println!("{:?}",c.unwrap())
    }
}
