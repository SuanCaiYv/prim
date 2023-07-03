use byteorder::{BigEndian, ByteOrder};

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
