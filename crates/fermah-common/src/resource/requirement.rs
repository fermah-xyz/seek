use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use super::gpu::GPUModel;
use crate::hash::Hashable;

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRequirement {
    pub min_vram: Option<u64>,
    pub min_ram: Option<u64>,
    pub min_ssd: Option<u64>,
    pub min_gpu: Vec<GPUModel>,
    pub min_cpu_cores: Option<u64>,
}

impl Hashable for ResourceRequirement {
    fn collect(&self) -> Cow<[u8]> {
        serde_json::to_vec(self).unwrap().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization() {
        let rrs = vec![
            ResourceRequirement {
                min_vram: Some(12 * 1024 * 1024 * 1024),
                min_ram: Some(16 * 1024 * 1024 * 1024),
                min_ssd: Some(16 * 1024 * 1024 * 1024),
                min_gpu: vec![GPUModel::GeForceRtx3060_12GB],
                min_cpu_cores: Some(16),
            },
            ResourceRequirement {
                min_vram: Some(4 * 1024 * 1024 * 1024),
                min_ram: Some(200 * 1024 * 1024 * 1024),
                min_ssd: Some(16 * 1024 * 1024 * 1024),
                min_gpu: vec![GPUModel::GeForceRtx3060_12GB],
                min_cpu_cores: Some(96),
            },
            ResourceRequirement {
                min_vram: None,
                min_ram: Some(200 * 1024 * 1024 * 1024),
                min_ssd: Some(16 * 1024 * 1024 * 1024),
                min_gpu: vec![GPUModel::GeForceRtx3060_12GB],
                min_cpu_cores: None,
            },
        ];

        let s = serde_json::to_string_pretty(&rrs).unwrap();

        println!("{}", s);

        let rs: Vec<ResourceRequirement> = serde_json::from_str(&s).unwrap();
        assert_eq!(rrs, rs);
        println!("{:?}", rs);
    }
}
