//! HOSHIZORA — HSZR-MBI  |  JSON Report Writer
//!
//! Writes timestamped injection results to ./reports/<unix_ts>.json

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::injector::InjectionResult;

fn unix_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn write_report(results: &[InjectionResult], output_dir: &str) -> Result<String, String> {
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Cannot create report dir '{}': {}", output_dir, e))?;

    let ts       = unix_ts();
    let filename = format!("{}/{}.json", output_dir.trim_end_matches('/'), ts);

    let success_count = results.iter().filter(|r| r.success).count();
    let failed_count  = results.len() - success_count;

    // Manually build JSON (avoid chrono/time deps)
    let results_json: Vec<String> = results
        .iter()
        .map(|r| {
            let err_field = match &r.error {
                Some(e) => format!("\"{}\"", e.replace('"', "'")),
                None    => "null".into(),
            };
            format!(
                "  {{\n    \"attack\": \"{}\",\n    \"fc\": {},\n    \"target\": \"{}\",\
                \n    \"success\": {},\n    \"response\": \"{}\",\n    \"error\": {}\n  }}",
                r.attack_name, r.function_code, r.target, r.success, r.response_hex, err_field
            )
        })
        .collect();

    let content = format!(
        "{{\n  \"timestamp\": {},\n  \"total\": {},\n  \"success\": {},\n  \"failed\": {},\
        \n  \"results\": [\n{}\n  ]\n}}\n",
        ts,
        results.len(),
        success_count,
        failed_count,
        results_json.join(",\n")
    );

    fs::write(&filename, &content)
        .map_err(|e| format!("Write '{}' failed: {}", filename, e))?;

    Ok(filename)
}
