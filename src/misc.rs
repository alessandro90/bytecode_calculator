pub fn u8_as_i8(byte: u8) -> i8 {
    unsafe { *(&byte as *const u8 as *const i8) }
}

pub fn i8_as_u8(byte: i8) -> u8 {
    unsafe { *(&byte as *const i8 as *const u8) }
}

#[macro_export]
macro_rules! assert_float_eq {
    ($a:expr, $b:expr) => {
        assert!($a.abs() >= $b.abs() - 1e-6 && $a.abs() <= $b.abs() + 1e-6)
    };
    ($a:expr, $b:expr, $delta:expr) => {
        assert!($a.abs() >= $b.abs() - $delta && $a.abs() <= $b.abs() + $delta)
    };
}
