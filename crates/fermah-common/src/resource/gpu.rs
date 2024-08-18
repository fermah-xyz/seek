use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use super::{
    memory::{Memory, MemoryType},
    traits::Fulfillable,
};

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum GPUModel {
    RadeonProW6600x,
    IntelArcA770,
    GeForceRtx2080MaxQ,
    RadeonRxVega56,
    RadeonRx6800M,
    RadeonVegaFrontierEdition,
    GeForceRtx3060Laptop,
    Rtx1000AdaGenerationLaptop,
    RadeonRx6700M,
    GeForceGtx1070,
    RadeonProW6600m,
    RadeonRx6600M,
    QuadroRtx5000MaxQ,
    NvidiaTitanX,
    RadeonRx5600XT,
    RadeonRx6600S,
    RtxA2000_12gb,
    RtxA2000,
    GeForceRtx2080SuperMaxQ,
    GeForceGtx980Ti,
    GeForceRtx2070SuperMaxQ,
    RadeonTRx6850mXt,
    RadeonProWX8200,
    RadeonRx7600S,
    GeForceRtx2060,
    RadeonRxVega64,
    RadeonRx5700,
    GeForceRtx4050Laptop,
    RtxA3000_12gbLaptop,
    RadeonProVega64X,
    NvidiaA40,
    RadeonProW7500,
    GeForceGtx1070Ti,
    QuadroRtx5000Mobile,
    RadeonRx6700S,
    RadeonRx6650M,
    GeForceRtx2080Mobile,
    RadeonRx6600,
    QuadroP6000,
    QuadroK2200,
    QuadroK4200,
    RadeonProW5700,
    GeForceRtx3060_8GB,
    QuadroRtx4000,
    Rtx2000AdaGenerationLaptop,
    RtxA4000laptop,
    RadeonRx6800S,
    GeForceRtx3070Laptop,
    GeForceGtx1080,
    RadeonProVegaII,
    RadeonRx7700S,
    RadeonProW6600,
    Rtx3000AdaGenerationLaptop,
    GeForceRtx2060_12GB,
    MiracastdisplayportdriverV3,
    RadeonProVegaIIDuo,
    QuadroRtx5000,
    RtxA5000laptop,
    GeForceRtx2070,
    TeslaV100SXM2_16GB,
    RadeonRx7600,
    QuadroGP100,
    GeForceRtx3080Laptop,
    RadeonRx6600XT,
    GeForceRtx2060SUPER,
    RadeonRx5700XT50thAnniversary,
    RadeonRx5700XT,
    RadeonVII,
    TitanVCeoEdition,
    M60,
    P4,
    P40,
    AmpereA2,
    T4,
    A16,
    A10,
    A10G,
    GeForceRtx3060_12GB,
    RadeonRx6850mXt,
    RadeonRx6650XT,
    Rtx2000AdaGeneration,
    RtxA5500laptop,
    RadeonRx7600XT,
    RadeonProW5700X,
    RtxA4500laptop,
    GeForceRtx4060Laptop,
    A40_48Q,
    RadeonProW7600,
    GeForceRtx3070TiLaptop,
    GeForceRtx2070SUPER,
    NvidiaA10G,
    NvidiaTitanXp,
    RadeonRx6750GRE12GB,
    GeForceGtx1080Ti,
    QuadroRtx6000,
    GeForceRtx2080,
    RadeonRx6700,
    TitanXpCollectorsEdition,
    RtxA4000,
    QuadroRtx8000,
    GeForceRtx4060,
    GeForceRtx2080SUPER,
    Rtx3500AdaGenerationLaptop,
    GeForceRtx4070Laptop,
    RadeonProW6800,
    QuadroGV100,
    RadeonRx6700XT,
    TtitanV,
    GeForceRtx3080TiLaptop,
    TitanRtx,
    GeForceRtx3060Ti,
    RadeonRx6750GRE10GB,
    RadeonRx6750XT,
    RadeonProW7700,
    Rtx4000sffAdaGeneration,
    RtxA5500,
    Radeon610mRyzen9_7845hx,
    GeForceRtx2080Ti,
    RtxA4500,
    GridRtx6000_6Q,
    RadeonRx7900M,
    NvidiaA10,
    RadeonRx7700XT,
    RadeonRx6800,
    GeForceRtx3070,
    RtxA6000,
    RtxA5000,
    GeForceRtx4060Ti16GB,
    GeForceRtx4060Ti,
    Rtx4000AdaGenerationLaptop,
    GeForceRtx3070Ti,
    RadeonRx7800XT,
    Rtx5000AdaGenerationLaptop,
    Rtx5000AdaGeneration,
    RadeonRx6800XT,
    GeForceRtx3080,
    GeForceRtx4080Laptop,
    Rtx4000AdaGeneration,
    RadeonRx7900GRE,
    GeForceRtx3080_12GB,
    GeForceRtx3090,
    RadeonRx6900XT,
    GeForceRtx4070,
    GeForceRtx3080Ti,
    Rtx6000AdaGeneration,
    GeForceRtx4090Laptop,
    RadeonRx6950XT,
    RadeonRx7900XT,
    RadeonProW7800,
    RadeonProW7900,
    GeForceRtx3090Ti,
    Rtx4500AdaGeneration,
    GeForceRtx4070SUPER,
    RadeonRx7900XTX,
    L4,
    V100,
    V100S,
    GA100Ampere,
    A100,
    L40S,
    GeForceRtx4070TiSUPER,
    GeForceRtx4070Ti,
    GeForceRtx4080SUPER,
    GeForceRtx4080,
    GeForceRtx4090D,
    GeForceRtx4090,
    H100,
}

// #[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
// pub enum GPUMemoryType {

// }

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct GPUMemory {
    pub size: u64,
    pub r#type: MemoryType,
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GPUSpecs {
    pub cores: u64,
    pub memory: Memory,
    /// Clock rate in HZ
    pub clock_rate: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum GPU {
    Model(GPUModel),
    Specs(GPUSpecs),
}

impl PartialOrd for GPU {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GPU {
    fn cmp(&self, other: &Self) -> Ordering {
        if let Self::Model(m) = self {
            if let Self::Model(m_other) = other {
                return m.cmp(m_other);
            }
        }

        let self_specs = self.specs();
        let other_specs = other.specs();

        self_specs
            .memory
            .cmp(&other_specs.memory)
            .then(self_specs.cores.cmp(&other_specs.cores))
            .then(self_specs.clock_rate.cmp(&other_specs.clock_rate))
    }
}

impl GPUModel {
    pub fn specs(&self) -> &GPUSpecs {
        match self {
            Self::GeForceRtx3060_12GB => {
                &GPUSpecs {
                    cores: 3_584,
                    memory: Memory {
                        size: 12 * 1024 * 1024 * 1024,
                        r#type: MemoryType::GDDR6,
                    },
                    clock_rate: 1_320_000_000,
                }
            }
            Self::GeForceRtx3060_8GB => {
                &GPUSpecs {
                    cores: 3_584,
                    memory: Memory {
                        size: 8 * 1024 * 1024 * 1024,
                        r#type: MemoryType::GDDR6,
                    },
                    clock_rate: 1_320_000_000,
                }
            }
            Self::GeForceRtx3060Ti => {
                &GPUSpecs {
                    cores: 4_864,
                    memory: Memory {
                        size: 8 * 1024 * 1024 * 1024,
                        r#type: MemoryType::GDDR6X,
                    },
                    clock_rate: 1_410_000_000,
                }
            }
            _ => {
                &GPUSpecs {
                    cores: 4_864,
                    memory: Memory {
                        size: 8 * 1024 * 1024 * 1024,
                        r#type: MemoryType::GDDR6X,
                    },
                    clock_rate: 1_410_000_000,
                }
            }
        }
    }
}

impl GPU {
    pub fn specs(&self) -> &GPUSpecs {
        match self {
            Self::Model(m) => m.specs(),
            Self::Specs(s) => s,
        }
    }
}

impl Default for GPU {
    fn default() -> Self {
        Self::Model(GPUModel::GeForceRtx3060_8GB)
    }
}

impl Fulfillable<GPUModel> for GPUModel {
    fn fulfills(&self, other: &Self) -> bool {
        self >= other
    }
}

impl Fulfillable<GPUModel> for GPU {
    fn fulfills(&self, other: &GPUModel) -> bool {
        self.specs().memory.size >= other.specs().memory.size
    }
}

impl Fulfillable<GPU> for GPU {
    fn fulfills(&self, other: &Self) -> bool {
        // other parameters are soft, so it is up to Ord to compare them in power
        self.specs().memory.size >= other.specs().memory.size
    }
}
