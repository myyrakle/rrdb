macro_rules! join_vec {
    ($x:expr) => {
        $x
    };
    ($x:expr, $($y:expr),+) => {
        |mut lhs: Vec<_>, mut rhs: Vec<_>|->Vec<_>{lhs.append(&mut rhs);
            lhs}($x, join_vec!($($y),+))
    };
}

pub(crate) use join_vec;
