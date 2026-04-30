pub enum SecurityLevel {
    Low,
    Medium,
    High,
}

pub fn default_security_level() -> SecurityLevel {
    SecurityLevel::Low
}
