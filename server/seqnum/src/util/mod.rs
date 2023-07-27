use std::{io::{Write, Read}, path::PathBuf};

use byteorder::{BigEndian, ByteOrder};
use tracing::warn;
use lib::Result;

pub(crate) static mut MY_ID: u32 = 0;

#[inline]
pub(crate) fn my_id() -> u32 {
    unsafe { MY_ID }
}

pub(crate) fn load_my_id(my_id_preload: u32) -> Result<()> {
    if my_id_preload == 0 {
        let path = PathBuf::from("./seqnum/my_id");
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
        if let Err(e) = std::fs::remove_file("./seqnum/my_id") {
            warn!("remove my_id file error: {}", e);
        }
        if let Err(e) = std::fs::create_dir_all("./seqnum") {
            warn!("create seqnum dir error: {}", e);
        }
        let mut file = match std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open("./seqnum/my_id") {
                Ok(file) => file,
                Err(e) => {
                    warn!("create my_id file error: {}", e);
                    return Err(e.into());
                }
        };
        if let Err(e) = file.write_all(my_id_preload.to_string().as_bytes()) {
            warn!("write my_id file error: {}", e);
            return Err(e.into());
        }
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
