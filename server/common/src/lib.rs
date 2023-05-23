pub mod scheduler;

#[cfg(test)]
mod tests {
    use std::{vec, println};


    #[test]
    fn it_works() {
        let val1: &Vec<u8> = &vec![b't', b'r', b'u', b'e'];
        let val2 = b"true";
        println!("{}", val1 == val2);
    }
}
