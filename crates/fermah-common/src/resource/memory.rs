use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use super::traits::Fulfillable;

pub const KILO_BYTE: u64 = 1024;
pub const MEGA_BYTE: u64 = 1024 * KILO_BYTE;
pub const GIGA_BYTE: u64 = 1024 * MEGA_BYTE;

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum RAMMemoryType {
    DDR3,
    DDR4,
    DDR5,
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum SSDMemoryType {
    SATAIII,
    NVMeGen3,
    NVMeGen4,
    NVMeGen5,
    U2,
    M2,
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Memory<T> {
    pub size: u64,
    pub r#type: T,
}

impl<T> Fulfillable<Memory<T>> for Memory<T> {
    fn fulfills(&self, other: &Self) -> bool {
        self.size >= other.size
    }
}

impl Default for Memory<RAMMemoryType> {
    fn default() -> Self {
        Self {
            size: 16 * 1024 * 1024 * 1024,
            r#type: RAMMemoryType::DDR5,
        }
    }
}

impl Default for Memory<SSDMemoryType> {
    fn default() -> Self {
        Self {
            size: 128 * 1024 * 1024 * 1024,
            r#type: SSDMemoryType::NVMeGen3,
        }
    }
}

impl<T: Ord + PartialOrd> Ord for Memory<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.size
            .cmp(&other.size)
            .then(self.r#type.cmp(&other.r#type))
    }
}

impl<T: Ord + PartialEq> PartialOrd for Memory<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
