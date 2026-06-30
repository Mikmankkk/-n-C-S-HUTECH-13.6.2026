# Hoshizora SCADA Attack Simulator

> **Educational ICS/SCADA security research platform focused on Modbus TCP injection detection.**

---

## Architecture

```
[ HSZR-SCADA ]  192.168.10.10   ← Debian VM  (Modbus Master + API)
      |
      ↓  Authorized FC03/FC06 polling
[ SURICATA ]    br0              ← Gateway VM  (IDS/IPS bridge)
      |    ↑
      |    └──── [ HSZR-MBI ]   192.168.10.66  ← Attacker VM (Rust injector)
      ↓              FC06 / FC16 inject
[ HSZR-IOT ]   192.168.10.20   ← ESP32-S3 Heltec  (Modbus Slave)
```

## Components

| Dir | Component | Language | Role |
|-----|-----------|----------|------|
| `hszr-scada/` | SCADA Control Center | Python | Modbus Master + FastAPI + Dashboard server |
| `hszr-iot/`   | Edge IoT Device       | C++ (Arduino) | Modbus Slave on ESP32-S3 Heltec |
| `hszr-mbi/`   | Modbus Injector       | Rust | Rogue attacker CLI tool |
| `suricata/`   | IDS/IPS Gateway       | YAML/rules | Suricata bridge config |
| `dashboard/`  | Web Monitor           | HTML/CSS/JS | Real-time register + alert dashboard |

---

## Quick Start

### 1. HSZR-IOT (ESP32)
```bash
cd hszr-iot
# Edit src/main.cpp: set WIFI_SSID, WIFI_PASS
pio run -t upload
```

### 2. HSZR-SCADA (Debian VM)
```bash
cd hszr-scada
pip install -r requirements.txt
python src/main.py        # API on :8000, dashboard on http://192.168.10.10:8000/
```

### 3. Suricata (Gateway VM)
```bash
cd suricata
sudo bash setup.sh eth1 eth2    # eth1=MBI side, eth2=IoT side
```

### 4. HSZR-MBI (Attacker VM)
```bash
cd hszr-mbi
cargo build --release
./target/release/hszr-mbi scan 192.168.10.0/24
./target/release/hszr-mbi inject
./target/release/hszr-mbi inject --attack "Pump"
```

### 5. Dashboard
Open `http://192.168.10.10:8000/` in a browser on the host.

---

## Register Map

| Address | Name           | Description          |
|---------|----------------|----------------------|
| 0x0001  | PUMP_CONTROL   | 0=Off, 1=On          |
| 0x0002  | VALVE_STATUS   | 0=Closed, 1=Open     |
| 0x0003  | ALARM_OVERRIDE | 0=Normal, 1=Alarm    |
| 0x0004  | TEMPERATURE    | °C × 10              |
| 0x0005  | PRESSURE       | kPa × 10             |

---

## Disclaimer
This project is for **educational and authorized security research purposes only**.




