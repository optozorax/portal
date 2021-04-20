use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorageWithNames<T> {
    pub names: Vec<String>,
    pub storage: Vec<T>,
}
