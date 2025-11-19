//! JSON formatter for analysis results.

use danny_core::AnalysisResult;
use serde_json::json;

pub struct JsonFormatter;

pub fn print_json(result: &AnalysisResult) {
    // Create a JSON structure with category added to each finding
    let json_result = json!({
        "findings": result.findings.iter().map(|f| {
            let mut finding_json = serde_json::to_value(f).unwrap();
            if let Some(obj) = finding_json.as_object_mut() {
                obj.insert("category".to_string(), json!(f.category()));
            }
            finding_json
        }).collect::<Vec<_>>(),
        "statistics": result.statistics,
        "errors": result.errors,
        "ignored_findings": result.ignored_findings,
    });

    match serde_json::to_string_pretty(&json_result) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Error serializing results: {}", e),
    }
}
