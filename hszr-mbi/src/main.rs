//! HOSHIZORA — HSZR-MBI  |  Main CLI Orchestrator
//!
//! USAGE
//!   hszr-mbi scan   <target>                 Discover Modbus slaves
//!   hszr-mbi inject [--attack <name>]        Run attacks from config.json
//!   hszr-mbi read   <register>               FC03 read single register
//!   hszr-mbi write  <register> <value>       FC06 write single register
//!   hszr-mbi help                            Show help

mod config;
mod modbus;
mod scanner;
mod injector;
mod reporter;

use std::env;

// ─── Banner ───────────────────────────────────────────────────────────────────

const BANNER: &str = "\
\x1b[36m
  ██╗  ██╗███████╗███████╗██████╗       ███╗   ███╗██████╗ ██╗
  ██║  ██║╚══███╔╝██╔══██╗██╔══██╗      ████╗ ████║██╔══██╗██║
  ███████║  ███╔╝ ██████╔╝██████╔╝█████╗██╔████╔██║██████╔╝██║
  ██╔══██║ ███╔╝  ██╔══██╗██╔══██╗╚════╝██║╚██╔╝██║██╔══██╗██║
  ██║  ██║███████╗███████║██║  ██║      ██║ ╚═╝ ██║██████╔╝██║
  ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝  ╚═╝      ╚═╝     ╚═╝╚═════╝ ╚═╝
\x1b[0m  HSZR-MBI v1.0.0  |  ICS/OT Modbus Injector  |  \x1b[33m[EDUCATIONAL]\x1b[0m
  ─────────────────────────────────────────────────────────────
";

const HELP: &str = "\
\x1b[1mUSAGE:\x1b[0m
  hszr-mbi [--config <path>] <command> [options]

\x1b[1mCOMMANDS:\x1b[0m
  scan   <target>              Discover Modbus slaves (IP or CIDR)
  inject [--attack <name>]     Run attacks from config.json
  read   <register>            FC03 — read a single holding register
  write  <register> <value>    FC06 — write a single holding register
  help                         Show this help

\x1b[1mOPTIONS:\x1b[0m
  --config <path>   Config file (default: config.json)
  --attack <name>   Filter attacks by name substring (inject only)

\x1b[1mEXAMPLES:\x1b[0m
  hszr-mbi scan 192.168.10.0/24
  hszr-mbi inject
  hszr-mbi inject --attack \"Pump\"
  hszr-mbi read 1
  hszr-mbi write 3 1
  hszr-mbi --config /etc/hszr/config.json inject
";

// ─── Arg parser ───────────────────────────────────────────────────────────────

struct Args {
    config_path:  String,
    command:      String,
    positional:   Vec<String>,
    attack_filter: Option<String>,
}

fn parse_args() -> Args {
    let raw: Vec<String> = env::args().skip(1).collect();
    let mut config_path   = "config.json".to_string();
    let mut attack_filter = None;
    let mut positional    = Vec::new();
    let mut i = 0;

    while i < raw.len() {
        match raw[i].as_str() {
            "--config" if i + 1 < raw.len() => {
                config_path = raw[i + 1].clone();
                i += 2;
            }
            "--attack" if i + 1 < raw.len() => {
                attack_filter = Some(raw[i + 1].clone());
                i += 2;
            }
            _ => {
                positional.push(raw[i].clone());
                i += 1;
            }
        }
    }

    let command = positional.first().cloned().unwrap_or_default();
    Args { config_path, command, positional, attack_filter }
}

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    println!("{}", BANNER);
    let args = parse_args();

    match args.command.as_str() {

        // ── Help ─────────────────────────────────────────────────────────────
        "help" | "--help" | "-h" | "" => println!("{}", HELP),

        // ── Scan ─────────────────────────────────────────────────────────────
        "scan" => {
            let target = match args.positional.get(1) {
                Some(t) => t.clone(),
                None => {
                    eprintln!("[ERROR] scan requires a target.  Usage: hszr-mbi scan <IP|CIDR>");
                    std::process::exit(1);
                }
            };
            let cfg = config::load_config(&args.config_path).unwrap_or_default();
            print_section("SCAN", &format!("Target: {}   Port: {}", target, cfg.port));

            let hosts = scanner::expand_cidr(&target);
            println!("[*] Probing {} host(s)...\n", hosts.len());

            let mut found = 0usize;
            for ip in &hosts {
                print!("  {} ... ", ip);
                match scanner::probe_host(ip, cfg.port, 500) {
                    None => println!("closed"),
                    Some(t) if t.is_modbus => {
                        found += 1;
                        let regs = t.registers
                            .map(|r| format!("{:?}", r))
                            .unwrap_or_else(|| "n/a".into());
                        println!("\x1b[32mMODBUS SLAVE\x1b[0m  regs[1-5]={}", regs);
                    }
                    Some(_) => println!("open (no Modbus)"),
                }
            }
            println!("\n[*] Done — {} Modbus slave(s) found.", found);
        }

        // ── Inject ───────────────────────────────────────────────────────────
        "inject" => {
            let cfg = config::load_config(&args.config_path).unwrap_or_else(|e| {
                eprintln!("[ERROR] {}", e);
                std::process::exit(1);
            });
            print_section("INJECT", &format!(
                "Target: {}:{}  UnitID: {}  Delay: {}ms  Attacks: {}",
                cfg.target, cfg.port, cfg.unit_id, cfg.delay_ms, cfg.attacks.len()
            ));

            let filter = args.attack_filter.as_deref();
            let results = injector::run_all(&cfg, filter);

            let ok  = results.iter().filter(|r| r.success).count();
            let tot = results.len();
            println!("\n\x1b[1m[SUMMARY]\x1b[0m  {} / {} attacks succeeded", ok, tot);

            match reporter::write_report(&results, &cfg.report_output) {
                Ok(path)  => println!("[*] Report → {}", path),
                Err(e)    => eprintln!("[WARN] Report failed: {}", e),
            }
        }

        // ── Read ─────────────────────────────────────────────────────────────
        "read" => {
            let reg: u16 = args.positional.get(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| { eprintln!("[ERROR] Usage: hszr-mbi read <register>"); std::process::exit(1); });

            let cfg = config::load_config(&args.config_path).unwrap_or_default();
            print_section("READ", &format!("FC03  reg={}  target={}:{}", reg, cfg.target, cfg.port));

            let frame = modbus::build_fc03(cfg.unit_id, reg, 1);
            modbus::hexdump("TX", &frame);

            match modbus::send_and_recv(&cfg.target, cfg.port, &frame, 2000) {
                Ok(resp) => {
                    modbus::hexdump("RX", &resp);
                    match modbus::parse_fc03_response(&resp) {
                        Some(v) => println!("[*] Register {}: \x1b[33m{}\x1b[0m", reg, v[0]),
                        None    => println!("[WARN] Could not parse response"),
                    }
                }
                Err(e) => eprintln!("[ERROR] {}", e),
            }
        }

        // ── Write ────────────────────────────────────────────────────────────
        "write" => {
            let reg: u16 = args.positional.get(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| { eprintln!("[ERROR] Usage: hszr-mbi write <reg> <val>"); std::process::exit(1); });
            let val: u16 = args.positional.get(2)
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| { eprintln!("[ERROR] Usage: hszr-mbi write <reg> <val>"); std::process::exit(1); });

            let cfg = config::load_config(&args.config_path).unwrap_or_default();
            print_section("WRITE", &format!("FC06  reg={}  val={}  target={}:{}", reg, val, cfg.target, cfg.port));

            let frame = modbus::build_fc06(cfg.unit_id, reg, val);
            modbus::hexdump("TX", &frame);

            match modbus::send_and_recv(&cfg.target, cfg.port, &frame, 2000) {
                Ok(resp) => {
                    modbus::hexdump("RX", &resp);
                    if !modbus::is_exception(&resp) {
                        println!("[*] \x1b[32mWrite succeeded\x1b[0m");
                    } else {
                        println!("[*] \x1b[31mException response\x1b[0m  code=0x{:02X}", resp.get(8).copied().unwrap_or(0));
                    }
                }
                Err(e) => eprintln!("[ERROR] {}", e),
            }
        }

        _ => {
            eprintln!("[ERROR] Unknown command: '{}'", args.command);
            println!("{}", HELP);
            std::process::exit(1);
        }
    }
}

fn print_section(name: &str, detail: &str) {
    println!("\x1b[1m[{}]\x1b[0m  {}", name, detail);
    println!("  {}", "─".repeat(60));
}
