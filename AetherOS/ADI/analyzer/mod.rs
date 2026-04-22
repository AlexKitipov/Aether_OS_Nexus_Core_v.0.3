pub struct Analyzer;

impl Analyzer {
    pub fn analyze(_src: &str) -> AnalyzedDriver {
        AnalyzedDriver::default()
    }
}

#[derive(Default)]
pub struct AnalyzedDriver;
