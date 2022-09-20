macro_rules! join_vec {
    ($x:expr) => {
        $x
    };
    ($x:expr, $($y:expr),+) => {
        join_vec_impl($x, join_vec!($($y),+))
    };
}

pub(crate) use join_vec;

pub(crate) fn join_vec_impl<T>(mut lhs: Vec<T>, mut rhs: Vec<T>) -> Vec<T> {
    lhs.append(&mut rhs);
    lhs
}
