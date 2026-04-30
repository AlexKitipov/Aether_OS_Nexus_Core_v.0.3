use super::{CapabilityMap, SecurityLevel};

pub struct PolicyDecision {
    pub allowed: bool,
}

pub fn enforce_policy(_cap: &CapabilityMap, _level: SecurityLevel) -> PolicyDecision {
    PolicyDecision { allowed: true }
}
