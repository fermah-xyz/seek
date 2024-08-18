use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use super::traits::Fulfillable;

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryType {
    DDR3,
    DDR4,
    DDR5,

    // GPU
    GDDR6,
    GDDR6X,
    HBM2,
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Memory {
    pub size: u64,
    pub r#type: MemoryType,
}

impl Fulfillable<Memory> for Memory {
    fn fulfills(&self, other: &Self) -> bool {
        self.size >= other.size
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            size: 16 * 1024 * 1024 * 1024,
            r#type: MemoryType::DDR5,
        }
    }
}

impl Ord for Memory {
    fn cmp(&self, other: &Self) -> Ordering {
        self.size
            .cmp(&other.size)
            .then(self.r#type.cmp(&other.r#type))
    }
}

impl PartialOrd for Memory {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
