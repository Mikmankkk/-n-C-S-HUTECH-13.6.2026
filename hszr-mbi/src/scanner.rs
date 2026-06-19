//! HOSHIZORA — HSZR-MBI  |  Network Scanner
//!
//! TCP connect-scan a CIDR range on the Modbus port,
//! then fingerprint live hosts with an FC03 read to confirm Modbus.

use std::net::TcpStream;
use std::time::Duration;

use crate::modbus;

// ─── Result type ──────────────────────────────────────────────────────────────

#[derive(Debug)]
#[allow(dead_code)]
pub struct ModbusTarget {
    pub ip:        String,
    pub port:      u16,
    pub is_modbus: bool,
    pub registers: Option<Vec<u16>>,
}

// ─── Single host probe ────────────────────────────────────────────────────────

pub fn probe_host(ip: &str, port: u16, timeout_ms: u64) -> Option<ModbusTarget> {
    let addr    = format!("{}:{}", ip, port);
    let timeout = Duration::from_millis(timeout_ms);

    // 1. Fast TCP connect check
    let sock_addr = addr.parse().ok()?;
    if TcpStream::connect_timeout(&sock_addr, timeout).is_err() {
        return None; // port closed
    }

    // 2. Modbus fingerprint — FC03 read regs 1-5
    let frame = modbus::build_fc03(1, 1, 5);
    match modbus::send_and_recv(ip, port, &frame, timeout_ms) {
        Ok(resp) => {
            let regs = modbus::parse_fc03_response(&resp);
            Some(ModbusTarget {
                ip: ip.into(),
                port,
                is_modbus: regs.is_some(),
                registers: regs,
            })
        }
        Err(_) => Some(ModbusTarget {
            ip: ip.into(),
            port,
            is_modbus: false,
            registers: None,
        }),
    }
}

// ─── CIDR expander ────────────────────────────────────────────────────────────

/// Parse a CIDR string (e.g. "192.168.10.0/24") or single IP into a list.
/// Supports /8 through /32.
pub fn expand_cidr(cidr: &str) -> Vec<String> {
    if let Some(slash) = cidr.find('/') {
        let base_ip  = &cidr[..slash];
        let prefix: u32 = cidr[slash + 1..].parse().unwrap_or(32);

        let parts: Vec<u8> = base_ip.split('.')
            .filter_map(|p| p.parse().ok())
            .collect();
        if parts.len() != 4 { return vec![cidr.into()]; }

        let base      = u32::from_be_bytes([parts[0], parts[1], parts[2], parts[3]]);
        let host_bits = 32u32.saturating_sub(prefix);
        let n_hosts   = (1u32 << host_bits).saturating_sub(2);
        let network   = base & (!0u32 << host_bits);

        (1..=n_hosts).map(|i| {
            let bytes = (network + i).to_be_bytes();
            format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
        }).collect()
    } else {
        vec![cidr.into()]
    }
}
