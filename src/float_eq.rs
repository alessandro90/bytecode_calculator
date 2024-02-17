#[allow(unused_macros)]
macro_rules! assert_float_eq {
    ($a:expr, $b:expr) => {
        assert!($a.abs() >= $b.abs() - 1e-6 && $a.abs() <= $b.abs() + 1e-6)
    };
    ($a:expr, $b:expr, $delta:expr) => {
        assert!($a.abs() >= $b.abs() - $delta && $a.abs() <= $b.abs() + $delta)
    };
}

#[allow(unused_imports)]
pub(crate) use assert_float_eq;
