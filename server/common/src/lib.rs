pub mod scheduler;

#[cfg(test)]
mod tests {
    use std::{cell::UnsafeCell, sync::Arc, thread::spawn};


    #[test]
    fn it_works() {
        struct S {
            a: i32,
            b: UnsafeCell<i32>,
        }

        unsafe impl Send for S {}
        unsafe impl Sync for S {}

        let a = S {
            a: 1,
            b: UnsafeCell::new(2),
        };
        let a = Arc::new(a);
        let a1 = a.clone();
        spawn(move || {
            println!("a1.a: {}", a1.a);
            println!("a1.b: {}", unsafe { *a1.b.get() });
        });
    }
}
