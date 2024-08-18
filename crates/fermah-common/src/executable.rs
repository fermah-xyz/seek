use std::{borrow::Cow, collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{
    hash::Hashable,
    resources::{LocalResource, RemoteResource},
};

pub type ImageName = String;

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Image {
    // Uses the general docker
    Docker(ImageName),
    /// Image name, remote resource to load the image. todo: make it contain ID (docker's image hash) of the image, not Hash
    /// Note: we are not checking that remote resource contains the image with image name that is ImageName. RemoteResource is
    /// a reference where to find the image
    RemoteDocker((RemoteResource, ImageName)),
    // Dev only
    LocalDocker((LocalResource, ImageName)),
}

impl Image {
    pub fn name(&self) -> &str {
        match self {
            Self::Docker(name) => name,
            Self::RemoteDocker((_, name)) => name,
            Self::LocalDocker((_, name)) => {
                warn!("Local docker is for local development only!");
                name
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    File(RemoteResource),
    /// Take all this files, and put them into the target directory
    Files(Vec<(PathBuf, RemoteResource)>),
    /// Unzip a directory as a target directory
    UnZipDirectory(RemoteResource),
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InMount {
    // in host
    pub source: Source,
    // pub source_hash: Hash,
    // This has to be a directory; in machine
    pub target: PathBuf,
    // if the file is required for one time only, and not expected to be used for multiple proofs
    pub temporary: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ResultExtractor {
    File(PathBuf),
    /// Note: don't use exit codes >255, as it may (will) be handled wrongly. In my case docker returned (some_exit_code mod 256)
    NegativeExitCode(i64),
    RegexStdout(String),
    // Directory(PathBuf),
}

// Injecting a file is simple with docker - just mount a file, ejecting is trickier, because the file is not existing yet, so we need to do it in folders
impl ResultExtractor {
    pub fn mount_point(&self) -> Option<PathBuf> {
        match self {
            Self::File(path) => path.parent().map(PathBuf::from),
            Self::RegexStdout(_) => None,
            Self::NegativeExitCode(_) => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Injector {
    File(PathBuf),
    Directory(PathBuf),
    // TODO: Env var?
}

impl Injector {
    pub fn mount_point(&self) -> Option<PathBuf> {
        match self {
            Self::File(path) => Some(path.clone()),
            Self::Directory(path) => Some(path.clone()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash)]
#[serde(rename_all = "camelCase")]
pub enum ExtractedResult {
    /// 0 code and extractor is File
    Bytes(Vec<u8>),
    /// Not used for now
    ZipDirectory(Vec<u8>),
    /// 0 code
    Success,
    /// non 0 code, but extractor has some code specified to return this result
    NegativeResult,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Executable {
    pub image: Image,
    pub platform: Option<String>,
    pub in_mounts: Vec<InMount>,
    /// Information on where to extract the information (primarily used for Proof extarction; for Prover)
    pub result_extractor: Option<ResultExtractor>,
    /// information on where to inject the possible information (primarily used for Proof injection; for Verifier)
    pub injector: Option<Injector>,
    pub entrypoint: Vec<String>,
    pub cmd: Vec<String>,
    // todo: probably can get rid of the option, and on deserialization treat no value as empty HashMap
    pub env_vars: Option<HashMap<String, String>>,
    pub network_enabled: bool,
    pub privileged: bool,
}

impl Hashable for Executable {
    fn collect(&self) -> Cow<[u8]> {
        let mut buf = vec![];
        buf.extend_from_slice(&serde_json::to_vec(&self.image).unwrap());
        buf.extend_from_slice(&serde_json::to_vec(&self.in_mounts).unwrap());
        buf.extend_from_slice(&serde_json::to_vec(&self.result_extractor).unwrap());
        buf.extend_from_slice(&serde_json::to_vec(&self.injector).unwrap());
        buf.extend_from_slice(&serde_json::to_vec(&self.entrypoint).unwrap());
        if let Some(ev) = &self.env_vars {
            let mut ev = ev.iter().collect::<Vec<(&String, &String)>>();
            ev.sort();
            ev.iter().for_each(|(k, v)| {
                buf.extend_from_slice(k.as_bytes());
                buf.extend_from_slice(v.as_bytes());
            })
        } else {
            buf.extend_from_slice("ev".as_bytes());
        };
        Cow::Owned(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization() {
        let rrs = vec![
            Executable {
                image: Image::Docker("dummy_prover:latest".to_string()),
                platform: None,
                in_mounts: vec![],
                result_extractor: None,
                injector: None,
                entrypoint: vec![],
                cmd: vec![],
                env_vars: None,
                network_enabled: false,
                privileged: false,
            },
            // Executable {
            //     image: crate::executable::Image::RemoteDocker(
            //         (
            //             crate::resources::RemoteResource {
            //                 url: "http://localhost:8082/dummy_prover_latest.tar.gz".parse().unwrap(),
            //                 hash:  Hash::from_bytes([50, 235, 26, 34, 170, 83, 73, 153, 59, 164, 55, 11, 174, 204, 153, 4, 87, 3, 75, 158, 8, 187, 32, 156, 174, 44, 132, 64, 14, 121, 100, 140]),
            //             },
            //             "dummy_prover:latest".to_string()
            //         )
            //     ),
            //     in_mounts: vec![],
            //     result_extractor: None,
            //     injector: None,
            //     entrypoint: vec!["python".into()],
            //     env_vars: None
            // }
        ];

        let s = serde_json::to_string_pretty(&rrs).unwrap();

        println!("{}", s);

        let rs: Vec<Executable> = serde_json::from_str(&s).unwrap();
        assert_eq!(rrs, rs);
        println!("{:?}", rs);

        let x = bincode::serialize(&rs).unwrap();

        let x = bincode::deserialize::<Vec<Executable>>(&x).unwrap();

        assert_eq!(x, rs)
    }
}
