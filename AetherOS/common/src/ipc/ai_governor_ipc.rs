extern crate alloc;

use alloc::string::String;
use serde::{Deserialize, Serialize};

/// Runtime class used by the AI governor when assigning execution budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiPriority {
    Interactive,
    Background,
    Batch,
}

/// Request messages for an AI resource-governor V-Node.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiGovernorRequest {
    /// Reserve CPU quota for a model-runtime session.
    ReserveCpu {
        requester: String,
        priority: AiPriority,
        /// Requested quota in millicores (1000 == one full core).
        millicores: u32,
    },
    /// Release all budget associated with a requester.
    ReleaseCpu {
        requester: String,
    },
    /// Fetch the current global quota usage and configured cap.
    QueryCpuBudget,
}

/// Response messages returned by the AI resource-governor V-Node.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiGovernorResponse {
    Granted {
        requester: String,
        granted_millicores: u32,
    },
    Denied {
        requester: String,
        reason: String,
    },
    Released {
        requester: String,
    },
    CpuBudget {
        total_cap_millicores: u32,
        used_millicores: u32,
    },
    Error {
        message: String,
    },
}
