use std::{io::{Write, Read}, path::PathBuf};

use byteorder::{BigEndian, ByteOrder};
use lib::Result;

pub(crate) static mut MY_ID: u32 = 0;

#[inline]
pub(crate) fn my_id() -> u32 {
    unsafe { MY_ID }
}

pub(crate) fn load_my_id(my_id_preload: u32) -> Result<()> {
    if my_id_preload == 0 {
        let path = PathBuf::from("./my_id");
        let path = path.as_path();
        let file = std::fs::File::open(path);
        if let Ok(file) = file {
            let mut reader = std::io::BufReader::new(file);
            let mut s = String::new();
            reader.read_to_string(&mut s)?;
            let my_id = s.parse::<u32>()?;
            unsafe { MY_ID = my_id };
        } else {
            panic!("my_id file not found");
        }
    } else {
        _ = std::fs::remove_file("./my_id");
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open("./my_id")?;
        file.write_all(my_id_preload.to_string().as_bytes())?;
        unsafe { MY_ID = my_id_preload }
    }
    Ok(())
}

pub(crate) fn as_bytes(key: u128, seqnum: u64, buf: &mut [u8]) {
    BigEndian::write_u128(&mut buf[0..16], key);
    BigEndian::write_u64(&mut buf[16..24], seqnum);
}

pub(crate) fn from_bytes(buf: &[u8]) -> (u128, u64) {
    (
        BigEndian::read_u128(&buf[0..16]),
        BigEndian::read_u64(&buf[16..24]),
    )
}

#[inline]
#[allow(unused)]
pub(crate) fn type_name<T>(_: &T) -> &'static str {
    std::any::type_name::<T>()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {}
}
