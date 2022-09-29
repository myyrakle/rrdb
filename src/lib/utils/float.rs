use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    ops::{Add, Div, Mul, Neg, Sub},
};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, PartialOrd)]
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
