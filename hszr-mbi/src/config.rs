//! HOSHIZORA — HSZR-MBI  |  Modbus TCP Injector
//! Config loader — reads config.json at runtime.

use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct AttackDef {
    pub name:      String,
    pub fc:        u8,
    // FC06 fields
    pub register:  Option<u16>,
    pub value:     Option<u16>,
    // FC16 fields
    pub start_reg: Option<u16>,
    pub values:    Option<Vec<u16>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub target:        String,
    pub port:          u16,
    pub unit_id:       u8,
    pub delay_ms:      u64,
    pub report_output: String,
    pub attacks:       Vec<AttackDef>,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            target:        "192.168.10.20".into(),
            port:          502,
            unit_id:       1,
            delay_ms:      500,
            report_output: "./reports/".into(),
            attacks:       vec![],
        }
    }
}

pub fn load_config(path: &str) -> Result<AppConfig, String> {
    if !Path::new(path).exists() {
        eprintln!("[WARN] Config '{}' not found — using defaults", path);
        return Ok(AppConfig::default());
    }
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read '{}': {}", path, e))?;
    serde_json::from_str(&raw)
        .map_err(|e| format!("Invalid JSON in '{}': {}", path, e))
}
