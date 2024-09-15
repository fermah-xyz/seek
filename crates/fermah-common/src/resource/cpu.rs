use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use super::traits::Fulfillable;

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum CPUModel {
    // Default, yet it is not clear on how to use it properly
    IntelI3,
    Ryzen3,
    IntelI5,
    Ryzen5,
    IntelI7,
    Ryzen7,
    IntelI9,
    IntelXeon,
    Threadripper,
    EPYC,
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct CPUSpecs {
    pub cores: u64,
    /// Clock rate in HZ
    pub clock_rate: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CPU {
    Model(CPUModel),
    Specs(CPUSpecs),
}

impl PartialOrd for CPU {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CPU {
    fn cmp(&self, other: &Self) -> Ordering {
        if let Self::Model(m) = self {
            if let Self::Model(m_other) = other {
                return m.cmp(m_other);
            }
        }

        let self_specs = self.specs();
        let other_specs = other.specs();

        self_specs
            .cores
            .cmp(&other_specs.cores)
            .then(self_specs.clock_rate.cmp(&other_specs.clock_rate))
    }
}

impl CPUModel {
    pub fn specs(&self) -> &CPUSpecs {
        match self {
            Self::Ryzen7 => {
                &CPUSpecs {
                    cores: 8,
                    clock_rate: 3_800_000_000,
                }
            }
            _ => {
                &CPUSpecs {
                    cores: 8,
                    clock_rate: 3_800_000_000,
                }
            }
        }
    }
}

impl CPU {
    pub fn specs(&self) -> &CPUSpecs {
        match self {
            Self::Model(m) => m.specs(),
            Self::Specs(s) => s,
        }
    }
}

impl Default for CPU {
    fn default() -> Self {
        Self::Model(CPUModel::Ryzen7)
    }
}

impl Fulfillable<CPUModel> for CPUModel {
    fn fulfills(&self, _other: &Self) -> bool {
        true
    }
}

impl Fulfillable<CPU> for CPU {
    fn fulfills(&self, _other: &Self) -> bool {
        // other parameters are soft, so it is up to Ord to compare them in power
        true
    }
}
