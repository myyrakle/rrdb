use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    ops::{Add, Div, Mul, Neg, Sub},
};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
pub struct Float64 {
    pub value: f64,
}

impl Eq for Float64 {}

impl Hash for Float64 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.to_ne_bytes().hash(state)
    }
}

impl ToString for Float64 {
    fn to_string(&self) -> String {
        self.value.to_string()
    }
}

impl From<f64> for Float64 {
    fn from(value: f64) -> Self {
        Float64 { value }
    }
}

impl From<Float64> for f64 {
    fn from(value: Float64) -> Self {
        value.value
    }
}

impl Neg for Float64 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        (-self.value).into()
    }
}

impl Add for Float64 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let result = self.value + other.value;
        result.into()
    }
}

impl Sub for Float64 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let result = self.value - other.value;
        result.into()
    }
}

impl Mul for Float64 {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        let result = self.value * other.value;
        result.into()
    }
}

impl Div for Float64 {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        let result = self.value / other.value;
        result.into()
    }
}

impl PartialEq for Float64 {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl PartialOrd for Float64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Float64 {
    fn cmp(&self, other: &Self) -> Ordering {
        if self == other {
            Ordering::Equal
        } else {
            let lhs = self.value;
            let rhs = other.value;

            match lhs.partial_cmp(&rhs) {
                Some(order) => order,
                None => Ordering::Less,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_float64_hash() {
        use super::Float64;
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        Float64 { value: 1.0 }.hash(&mut hasher);
        let hash1 = hasher.finish();

        let mut hasher = DefaultHasher::new();
        Float64 { value: 1.0 }.hash(&mut hasher);
        let hash2 = hasher.finish();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_float64_to_string() {
        use super::Float64;

        let f = Float64 { value: 1.0 };
        assert_eq!(f.to_string(), "1");
    }

    #[test]
    fn test_float64_from_f64() {
        use super::Float64;

        let f: Float64 = 1.0.into();
        assert_eq!(f, Float64 { value: 1.0 });
    }

    #[test]
    fn test_float64_from_float64() {
        use super::Float64;

        let f: f64 = Float64 { value: 1.0 }.into();
        assert_eq!(f, 1.0);
    }

    #[test]
    fn test_float64_neg() {
        use super::Float64;

        let f = Float64 { value: 1.0 };
        assert_eq!(-f, Float64 { value: -1.0 });
    }

    #[test]
    fn test_float64_add() {
        use super::Float64;

        let f1 = Float64 { value: 1.0 };
        let f2 = Float64 { value: 2.0 };
        assert_eq!(f1 + f2, Float64 { value: 3.0 });
    }

    #[test]
    fn test_float64_sub() {
        use super::Float64;

        let f1 = Float64 { value: 1.0 };
        let f2 = Float64 { value: 2.0 };
        assert_eq!(f1 - f2, Float64 { value: -1.0 });
    }

    #[test]
    fn test_float64_mul() {
        use super::Float64;

        let f1 = Float64 { value: 1.0 };
        let f2 = Float64 { value: 2.0 };
        assert_eq!(f1 * f2, Float64 { value: 2.0 });
    }

    #[test]
    fn test_float64_div() {
        use super::Float64;

        let f1 = Float64 { value: 1.0 };
        let f2 = Float64 { value: 2.0 };
        assert_eq!(f1 / f2, Float64 { value: 0.5 });
    }

    #[test]
    fn test_float64_eq() {
        use super::Float64;

        let f1 = Float64 { value: 1.0 };
        let f2 = Float64 { value: 1.0 };
        assert_eq!(PartialEq::eq(&f1, &f2), true);
    }

    #[test]
    fn test_float64_partial_cmp() {
        use super::Float64;

        let f1 = Float64 { value: 1.0 };
        let f2 = Float64 { value: 2.0 };
        assert_eq!(
            PartialOrd::partial_cmp(&f1, &f2),
            Some(std::cmp::Ordering::Less)
        );

        let f1 = Float64 { value: 1.0 };
        let f2 = Float64 { value: 1.0 };
        assert_eq!(
            PartialOrd::partial_cmp(&f1, &f2),
            Some(std::cmp::Ordering::Equal)
        );

        let f1 = Float64 { value: 2.0 };
        let f2 = Float64 { value: 1.0 };
        assert_eq!(
            PartialOrd::partial_cmp(&f1, &f2),
            Some(std::cmp::Ordering::Greater)
        );
    }

    #[test]
    fn test_float64_cmp() {
        use super::Float64;

        let f1 = Float64 { value: 1.0 };
        let f2 = Float64 { value: 2.0 };
        assert_eq!(Ord::cmp(&f1, &f2), std::cmp::Ordering::Less);

        let f1 = Float64 { value: 1.0 };
        let f2 = Float64 { value: 1.0 };
        assert_eq!(Ord::cmp(&f1, &f2), std::cmp::Ordering::Equal);

        let f1 = Float64 { value: 2.0 };
        let f2 = Float64 { value: 1.0 };
        assert_eq!(Ord::cmp(&f1, &f2), std::cmp::Ordering::Greater);
    }
}
