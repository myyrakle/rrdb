use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
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
