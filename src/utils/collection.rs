macro_rules! join_vec {
    ($x:expr) => {
        $x
    };
    ($x:expr, $($y:expr),+) => {
        {
            let join_fn = |mut lhs: Vec<_>, mut rhs: Vec<_>| -> Vec<_> {
                lhs.append(&mut rhs);
                lhs
            };

            join_fn($x, join_vec!($($y),+))
        }

    };
}

pub(crate) use join_vec;

#[cfg(test)]
mod tests {
    #[test]
    fn test_join_vec() {
        let v1 = vec![1, 2, 3];
        let v2 = vec![4, 5, 6];

        assert_eq!(join_vec!(v1), vec![1, 2, 3]);
        assert_eq!(join_vec!(v1, v2), vec![1, 2, 3, 4, 5, 6]);
    }
}
