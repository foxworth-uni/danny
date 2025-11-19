//! Output formatters for Danny analysis results.

pub mod human;
pub mod json;

pub use human::HumanFormatter;
pub use json::JsonFormatter;

/// Trait for formatting analysis results
pub trait Formatter {
    /// Format and print the analysis results
    fn format(&self, result: &danny_core::AnalysisResult);
}

impl Formatter for HumanFormatter {
    fn format(&self, result: &danny_core::AnalysisResult) {
        human::print_results(result);
    }
}

impl Formatter for JsonFormatter {
    fn format(&self, result: &danny_core::AnalysisResult) {
        json::print_json(result);
    }
}
