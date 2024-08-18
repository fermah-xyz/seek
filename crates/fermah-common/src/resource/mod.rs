use std::{borrow::Cow, cmp::Ordering};

use serde::{Deserialize, Serialize};

use self::{
    cpu::CPU,
    gpu::GPU,
    memory::Memory,
    requirement::ResourceRequirement,
    traits::{Fulfillable, Price},
};
use crate::hash::Hashable;

pub mod cpu;
pub mod gpu;
pub mod memory;
pub mod requirement;
pub mod traits;

/// Resource claims for prover server.
#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    /// RAM in Bytes.
    pub ram: Memory,
    /// RAM in Bytes.
    pub ssd: Memory,
    /// GPU properties.
    pub gpus: Vec<GPU>,
    /// CPU properties.
    pub cpu: CPU,
}

impl PartialOrd for Resource {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Resource {}

impl Ord for Resource {
    fn cmp(&self, other: &Self) -> Ordering {
        // It is very much not clear how to order Operators by their GPUs. Example:
        //      Should many small GPUs be greater than single beefy one, or vice-versa?
        // So, will order by best GPU on the machine for now
        let my_best_gpu = self.gpus.iter().max();
        let other_best_gpu = other.gpus.iter().max();
        let gpu_ordering = if let Some(my_best_gpu) = my_best_gpu {
            if let Some(other_best_gpu) = other_best_gpu {
                my_best_gpu.cmp(other_best_gpu)
            } else {
                Ordering::Greater
            }
        } else if other_best_gpu.is_some() {
            Ordering::Less
        } else {
            Ordering::Equal
        };
        gpu_ordering
            .then(self.ram.cmp(&other.ram))
            .then(self.cpu.cmp(&other.cpu))
    }
}

impl Price for Resource {
    fn price(&self) -> f64 {
        // Fallback for now
        100.0
        // self.ram as f64 * self.gpu.vram() as f64 * self.cpu.cores as f64
    }
}

// impl Fulfillable<Resource> for Resource {
//     /// Checks if this this resource is grater or equal than the one compared to. Ie this machine can execute compared load
//     fn fulfills(&self, other: &Self) -> bool {
//         self.ram >= other.ram && self.gpu.fulfills(&other.gpu)
//     }
// }

impl Hashable for Resource {
    fn collect(&self) -> Cow<[u8]> {
        serde_json::to_vec(self).unwrap().into()
    }
}

impl Fulfillable<ResourceRequirement> for Resource {
    fn fulfills(&self, req: &ResourceRequirement) -> bool {
        if let Some(min_ram) = req.min_ram {
            if self.ram.size < min_ram {
                return false;
            }
        }

        if let Some(min_ssd) = req.min_ssd {
            if self.ssd.size < min_ssd {
                return false;
            }
        }

        if let Some(min_cpu_cores) = req.min_cpu_cores {
            if self.cpu.specs().cores < min_cpu_cores {
                return false;
            }
        }

        if let Some(min_vram) = req.min_vram {
            for gpu in self.gpus.iter() {
                if gpu.specs().memory.size < min_vram {
                    return false;
                }
            }
        }

        let mut fulfilled_gpu_is: Vec<usize> = vec![];

        for gpu_req in req.min_gpu.iter() {
            for (i, gpu) in self.gpus.iter().enumerate() {
                if fulfilled_gpu_is.contains(&i) {
                    continue;
                }
                if gpu.fulfills(gpu_req) {
                    fulfilled_gpu_is.push(i);
                    break;
                }
            }
        }
        if fulfilled_gpu_is.len() != req.min_gpu.len() {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::{cpu::CPUSpecs, gpu::GPUSpecs, memory::MemoryType};

    #[test]
    fn test_serialization() {
        let rs = vec![
            Resource {
                ram: Memory {
                    size: 16 * 1024 * 1024 * 1024,
                    r#type: MemoryType::DDR4,
                },
                ssd: Memory {
                    size: 16 * 1024 * 1024 * 1024,
                    r#type: MemoryType::DDR4,
                },
                gpus: vec![GPU::Specs(GPUSpecs {
                    cores: 3_584,
                    memory: Memory {
                        size: 8 * 1024 * 1024 * 1024,
                        r#type: MemoryType::GDDR6,
                    },
                    clock_rate: 1_320_000_000,
                })],
                cpu: CPU::Specs(CPUSpecs {
                    cores: 16,
                    clock_rate: 3_800_000_000,
                }),
            },
            Resource {
                ram: Memory {
                    size: 16 * 1024 * 1024 * 1024,
                    r#type: MemoryType::DDR4,
                },
                ssd: Memory {
                    size: 16 * 1024 * 1024 * 1024,
                    r#type: MemoryType::DDR4,
                },
                gpus: vec![GPU::Model(gpu::GPUModel::GeForceRtx3060_12GB)],
                cpu: CPU::Model(cpu::CPUModel::Ryzen7),
            },
        ];

        let s = serde_json::to_string_pretty(&rs).unwrap();

        println!("{}", s);

        let rs: Vec<Resource> = serde_json::from_str(&s).unwrap();
        println!("{:?}", rs);
    }
}
