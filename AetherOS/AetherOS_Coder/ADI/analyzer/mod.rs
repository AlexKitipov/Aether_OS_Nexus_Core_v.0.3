pub mod rules;
pub mod score;
pub mod report;

pub use rules::*;
pub use score::*;
pub use report::*;

pub struct Analyzer;

impl Analyzer {
    pub fn analyze(_src: &str) -> AnalyzedDriver {
        AnalyzedDriver::default()
    }
}

#[derive(Default)]
pub struct AnalyzedDriver;
