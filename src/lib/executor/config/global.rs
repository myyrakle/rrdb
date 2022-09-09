use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GlobalConfig {}

#[allow(clippy::derivable_impls)]
impl std::default::Default for GlobalConfig {
    fn default() -> Self {
        Self {}
    }
}
