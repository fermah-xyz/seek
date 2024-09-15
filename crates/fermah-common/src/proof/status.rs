use std::fmt::Display;

use serde::{Deserialize, Serialize};
use strum::Display;

use crate::{operator::OperatorId, proof::Proof};

#[derive(Serialize, Deserialize, Display, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
pub enum ProofStatus {
    /// Was just created in the system
    Created,
    /// Passed most basic checks like signature
    Accepted,

    /// Closed, primarely due to PRer returning their funds
    Cancelled,
    /// Rejected with explanation
    /// - Eligible for payment
    Rejected(String),

    /// Assigned to a specific prover. Note: not the most solid state and can change assignment, waits for Acknowledged to be solid assigned
    Assigned(OperatorId),
    /// Solid assigned state. Prover confirmed the assignment.
    AcknowledgedAssignment(OperatorId),

    /// Received proof, but test is not complete
    ProofBeingTested(Proof),

    /// Proved
    /// - Eligible for payment
    Proven(Proof),
}

impl ProofStatus {
    pub fn reject<R: Display>(reason: R) -> Self {
        Self::Rejected(reason.to_string())
    }

    pub fn is_final(&self) -> bool {
        matches!(self, Self::Cancelled | Self::Proven(_) | Self::Rejected(_))
    }

    pub fn to_const_str(&self) -> &'static str {
        match self {
            Self::Created => "Created",
            Self::Accepted => "Accepted",
            Self::Cancelled => "Cancelled",
            Self::Rejected(_) => "Rejected",
            Self::Assigned(_) => "Assigned",
            Self::AcknowledgedAssignment(_) => "AcknowledgedAssignment",
            Self::ProofBeingTested(_) => "ProofBeingTested",
            Self::Proven(_) => "Proven",
        }
    }
}
