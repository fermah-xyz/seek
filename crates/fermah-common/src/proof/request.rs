use std::borrow::Cow;

use chrono::{DateTime, Utc};
use ethers::types::Address;
use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::{
    executable::Executable,
    hash::{blake3::Blake3Hash, Hashable},
    resource::requirement::ResourceRequirement,
};

pub type ProofRequestId = Blake3Hash;

/// Proof request payload.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProofRequest {
    /// Requester eth address.
    pub requester: Option<Address>,
    /// Prover image description.
    pub prover: Executable,
    /// Verifier image description.
    pub verifier: Executable,
    // /// Input for prover.
    // pub input: Vec<u8>,
    /// Minimal resource claims for the prover server.
    pub resource_requirement: ResourceRequirement,
    /// Callback to return the proof or error to requester
    pub callback_url: Option<Url>,
    /// Deadline till when the proof request need to be fulfilled E2E
    pub deadline: Option<DateTime<Utc>>,
    /// Nonce
    #[serde(default)]
    pub nonce: u64,
}

impl Hashable for ProofRequest {
    fn collect(&self) -> Cow<[u8]> {
        let mut optionals = vec![];

        if let Some(report_url) = &self.callback_url {
            optionals.extend(report_url.as_str().as_bytes())
        }

        if let Some(d) = &self.deadline {
            optionals.extend(d.to_string().as_bytes())
        }

        let empty_vec: Vec<u8> = vec![];
        let req_bytes = match &self.requester {
            Some(req) => req.as_bytes(),
            None => &empty_vec,
        };

        [
            req_bytes,
            self.prover.collect().as_ref(),
            self.verifier.collect().as_ref(),
            self.resource_requirement.collect().as_ref(),
            optionals.as_ref(),
            self.nonce.to_be_bytes().as_ref(),
        ]
        .concat()
        .into()
    }
}
