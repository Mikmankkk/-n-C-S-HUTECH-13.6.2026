# Hoshizora SCADA Simulator — Usage & Lab Guide

This guide details how to set up the lab environment, deploy the various components, and conduct both attack (red team) and defense (blue team) exercises within the Hoshizora ICS/SCADA simulation environment.

---

## 1. Network Topology & Lab Setup

For the most authentic experience, it is recommended to run this environment using **VMware Workstation** or **Proxmox** with a dedicated bridge network, alongside the physical ESP32 device.

### Lab Addressing

| Node | IP Address | Subnet | Role |
| :--- | :--- | :--- | :--- |
| **HSZR-SCADA** | `192.168.10.10` | `/24` | Legitimate Modbus Master & Dashboard (Debian VM) |
| **HSZR-IOT** | `192.168.10.20` | `/24` | Physical Edge Device (Modbus Slave on ESP32-S3) |
| **HSZR-MBI** | `192.168.10.66` | `/24` | Rogue Injector Attacker (Kali/Debian VM) |
| **SURICATA** | `Transparent` | `br0` | IDS/IPS Bridge Gateway (Debian/Ubuntu VM) |

### Setting up the Suricata Bridge
To monitor and theoretically block traffic without modifying routing tables on the attacker or SCADA nodes, the Suricata node should be configured as a transparent Layer 2 Bridge.

1. Equip the **Suricata VM** with two network adapters (e.g., `eth1` and `eth2`).
2. Connect `eth1` to the segment containing the *Attacker (HSZR-MBI)* and *SCADA Master*.
3. Connect `eth2` to the segment containing the *ESP32 (HSZR-IOT)*.
4. Run the setup script on the Gateway VM:
   ```bash
   cd suricata
   sudo bash setup.sh eth1 eth2
   ```
   This creates `br0`, puts both interfaces into promiscuous mode, and starts Suricata monitoring the bridge.

---

## 2. Deploying the Components

### A. Deploying HSZR-IOT (The Edge Device)
The IoT device sits on the edge network and controls the physical inputs/outputs (simulated via OLED).

1. Connect the **Heltec ESP32-S3 V3** board via USB.
2. Edit `hszr-iot/src/main.cpp` and update `WIFI_SSID` and `WIFI_PASS` to match your lab's wireless access point. (Ensure the AP is bridged to the `eth2` segment if using a strict bridge setup, or just ensure it's on the `192.168.10.0/24` subnet).
3. Build and upload using PlatformIO:
   ```bash
   cd hszr-iot
   pio run -t upload
   ```
4. Once flashed, the OLED will display its boot sequence and acquired IP.

### B. Deploying HSZR-SCADA (The Control Center)
The SCADA master continuously polls the IoT device so the operator can view telemetry.

1. On the **SCADA VM** (`192.168.10.10`), install Python dependencies:
   ```bash
   python3 -m venv venv
   source venv/bin/activate
   pip install -r hszr-scada/requirements.txt
   ```
2. Start the API/Master daemon (ensure `config.yaml` points to `192.168.10.20`):
   ```bash
   python hszr-scada/src/main.py
   ```
3. Open a browser and navigate to `http://192.168.10.10:8000/`. You should see the dark industrial HUD with live register polling.

### C. Deploying HSZR-MBI (The Attacker)
The MBI (Modbus Injector) is a modular Rust tool that bypasses the SCADA system to communicate directly with the IoT edge.

1. On the **Attacker VM** (`192.168.10.66`), compile the Rust binary:
   ```bash
   cd hszr-mbi
   cargo build --release
   ```
2. The executable will be at `target/release/hszr-mbi`.

---

## 3. Running the Simulation

### The "Normal" State
Once SCADA and IOT are running, observe the Web Dashboard:
- The **Logs Panel** will show continuous `FC03` (Read Holding Registers) polling.
- The **Register Cards** will update with live data.
- The **ESP32 OLED** will increment its `Polls:` count continuously.

### Red Team: Executing Modbus Injections
From the **Attacker VM**, you act as an adversary who has gained network access to the `192.168.10.0/24` segment.

1. **Reconnaissance:** Scan the network for Modbus devices.
   ```bash
   ./hszr-mbi scan 192.168.10.0/24
   ```
   *Expected:* It will find `192.168.10.20:502` and pull its current register values.

2. **Ad-Hoc Writing:** Turn on the heavy pump (Register 1).
   ```bash
   ./hszr-mbi write 1 1 --ip 192.168.10.20
   ```

3. **Automated Attack Campaigns:** Execute a full structured attack using `config.json`.
   ```bash
   ./hszr-mbi inject
   ```
   Or target a specific sequence:
   ```bash
   ./hszr-mbi inject --attack "Alarm Suppress"
   ```

### Blue Team: Observing Defenses
1. **Device Level Defense (The ESP32):**
   When the attacker VM writes to the ESP32, the firmware matches the IP against the authorized SCADA IP.
   - Go look at the physical ESP32 OLED screen. The bottom row will loudly flash: `!!INJECTED 192.168.10.66`.

2. **Network Level Defense (Suricata & Dashboard):**
   Because traffic flows through `br0`:
   - Suricata evaluates the packets against `suricata/rules/modbus.rules`.
   - It flags unauthorized `FC06` (Single Write) and `FC16` (Multiple Write) packets originating from non-SCADA IPs.
   - It also flags semantic logic flaws, like setting `ALARM_OVERRIDE` to `0` during a high-temperature state (Alarm Suppression attack).
   - This alert is logged to `eve.json`, instantly parsed by `HSZR-SCADA`, and blasted over WebSockets to the Dashboard.
   - **Look at the Dashboard's Alert Panel:** The red alarms will cascade in real-time.

---

## 4. Transitioning from IDS to IPS (Prevention)
By default, the simulation is set to **IDS mode (Intrusion Detection)**, meaning the attacker's packets still reach the edge device.

To transition to active prevention:
1. Stop Suricata on the Gateway VM.
2. Edit `suricata/suricata.yaml` and set:
   ```yaml
   af-packet:
     - interface: br0
       copy-mode: ips       # Change from 'tap' to 'ips'
   ```
3. Edit `suricata/rules/modbus.rules` and change the relevant `alert` rules to `drop`:
   ```snort
   drop modbus any any -> $MODBUS_NET 502 (msg:"[HSZR] UNAUTH FC06 WRITE"...
   ```
4. Restart Suricata. Run the attacker script again.
5. Watch the dashboard—the attacker's scripts will hang or fail, and the ESP32 registers will remain untouched.
