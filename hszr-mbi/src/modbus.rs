//! HOSHIZORA — HSZR-MBI  |  Modbus TCP Frame Builder + Transport
//!
//! Builds raw Modbus TCP ADUs (MBAP + PDU) and provides a single
//! send_and_recv() function over a short-lived TCP connection.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

// ─── MBAP helper ─────────────────────────────────────────────────────────────

/// Serialize the 7-byte MBAP header.
/// length = 1 (unit_id) + pdu_len
fn mbap(transaction_id: u16, unit_id: u8, pdu_len: u16) -> [u8; 7] {
    let ti  = transaction_id.to_be_bytes();
    let len = (1u16 + pdu_len).to_be_bytes();
    [ti[0], ti[1], 0x00, 0x00, len[0], len[1], unit_id]
}

// ─── Frame builders ───────────────────────────────────────────────────────────

/// FC03 — Read Holding Registers
pub fn build_fc03(unit_id: u8, start_reg: u16, count: u16) -> Vec<u8> {
    let pdu = [
        0x03u8,
        (start_reg >> 8) as u8, start_reg as u8,
        (count >> 8) as u8,     count as u8,
    ];
    let mut frame = mbap(1, unit_id, pdu.len() as u16).to_vec();
    frame.extend_from_slice(&pdu);
    frame
}

/// FC06 — Write Single Register
pub fn build_fc06(unit_id: u8, register: u16, value: u16) -> Vec<u8> {
    let pdu = [
        0x06u8,
        (register >> 8) as u8, register as u8,
        (value >> 8) as u8,    value as u8,
    ];
    let mut frame = mbap(1, unit_id, pdu.len() as u16).to_vec();
    frame.extend_from_slice(&pdu);
    frame
}

/// FC16 — Write Multiple Registers
pub fn build_fc16(unit_id: u8, start_reg: u16, values: &[u16]) -> Vec<u8> {
    let qty        = values.len() as u16;
    let byte_count = (qty * 2) as u8;
    let mut pdu = vec![
        0x10u8,
        (start_reg >> 8) as u8, start_reg as u8,
        (qty >> 8) as u8,       qty as u8,
        byte_count,
    ];
    for &v in values {
        pdu.push((v >> 8) as u8);
        pdu.push(v as u8);
    }
    let mut frame = mbap(1, unit_id, pdu.len() as u16).to_vec();
    frame.extend_from_slice(&pdu);
    frame
}

// ─── Transport ────────────────────────────────────────────────────────────────

/// Open a TCP connection, send frame, read response.
/// Uses a fresh connection per call (stateless, mirrors most Modbus injectors).
pub fn send_and_recv(host: &str, port: u16, frame: &[u8], timeout_ms: u64) -> Result<Vec<u8>, String> {
    let addr    = format!("{}:{}", host, port);
    let timeout = Duration::from_millis(timeout_ms);

    let mut stream = TcpStream::connect_timeout(
        &addr.parse().map_err(|e| format!("Bad address '{}': {}", addr, e))?,
        timeout,
    )
    .map_err(|e| format!("Connect → {}: {}", addr, e))?;

    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();

    stream.write_all(frame).map_err(|e| format!("TX error: {}", e))?;

    let mut buf = vec![0u8; 512];
    let n = stream.read(&mut buf).map_err(|e| format!("RX error: {}", e))?;
    buf.truncate(n);
    Ok(buf)
}

// ─── Utilities ────────────────────────────────────────────────────────────────

pub fn hexdump(label: &str, data: &[u8]) {
    let hex: Vec<String> = data.iter().map(|b| format!("{:02X}", b)).collect();
    println!("    {:4} [{}]", label, hex.join(" "));
}

/// Parse FC03 response → register values
pub fn parse_fc03_response(resp: &[u8]) -> Option<Vec<u16>> {
    // MBAP(7) + FC(1) + ByteCount(1) + data
    if resp.len() < 9 || resp[7] != 0x03 { return None; }
    let byte_count = resp[8] as usize;
    if resp.len() < 9 + byte_count      { return None; }
    Some(
        (0..byte_count / 2)
            .map(|i| ((resp[9 + i * 2] as u16) << 8) | resp[10 + i * 2] as u16)
            .collect(),
    )
}

/// True if the ADU contains a Modbus exception response
pub fn is_exception(resp: &[u8]) -> bool {
    resp.len() >= 8 && (resp[7] & 0x80) != 0
}
