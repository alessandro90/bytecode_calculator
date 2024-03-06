pub fn u8_as_i8(byte: u8) -> i8 {
    unsafe { *(&byte as *const u8 as *const i8) }
}

pub fn i8_as_u8(byte: i8) -> u8 {
    unsafe { *(&byte as *const i8 as *const u8) }
}
