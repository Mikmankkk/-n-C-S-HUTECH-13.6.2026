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



THIẾT KẾ VÀ XÂY DỰNG HỆ THỐNG
3.1. Tổng quan kiến trúc
Quyết định thiết kế quan trọng nhất của đồ án là dùng transparent bridge cho Suricata thay vì đặt nó tại một điểm cụ thể trong topology. Transparent bridge Layer 2 bridge nghĩa là Suricata "invisible" hoàn toàn với các host khác, cả SCADA lẫn kẻ tấn công đều không biết Suricata tồn tại. Gói tin từ HSZR MBI đến ESP32 đi qua br0 như thể đó là cable thẳng, nhưng thực ra Suricata đang sao chép IDS hoặc quyết định forward/drop từng packet.
Lợi thế thực tế: không cần thay đổi bất kỳ cấu hình nào trên các máy khác khi thêm Suricata. Không cần đổi default gateway, không cần thêm route. Trong môi trường production OT, đây là lợi thế lớn vì OT rất dị ứng với bất kỳ thay đổi cấu hình nào ngay cả thay đổi nhỏ cũng cần qua nhiều bước approval.
Thành phần	Phần cứng/OS	Stack	Nhiệm vụ chính
HSZR-SCADA	Debian 12 VM, 2GB RAM	Python 3.11, pymodbus 3.6.9, FastAPI 0.110, WebSocket	Poll Modbus FC03, REST API, WebSocket server, đọc eve.json
HSZR-IOT	Heltec ESP32-S3 V3 (vật lý)	FreeRTOS, C++/Arduino, ThingPulse SSD1306 4.4.1	Modbus TCP slave, IP whitelist check, OLED alert display
HSZR-MBI	Kali Linux 2024, 2GB RAM	Rust 2021, serde 1.0, serde_json 1.0	Attack CLI: scan / write / inject
Suricata GW	Ubuntu 22.04 VM, 2GB RAM, 2 NIC	Suricata 7.0.3, af-packet, modbus rules	Transparent IDS/IPS giữa MBI và IoT
Bảng 3. 1. Cấu hình chi tiết các thành phần hệ thống SCADA
3.2. Xây dựng môi trường thực nghiệm.
-	Thiết lập hệ thống mô phỏng sử dụng Modbus PLC giả lập, SCADA hoặc phần mềm mô phỏng. 
-	Sử dụng các công cụ như Wireshark, Modbus Simulator, hoặc môi trường ảo hóa VMware/Docker. 
-	Đảm bảo môi trường thử nghiệm tách biệt, không ảnh hưởng đến hệ thống thực tế. 
3.3. Thực nghiệm các kịch bản tấn công Modbus Injection.
-	Unauthorized Write: Gửi lệnh ghi trái phép vào thanh ghi coil/register để thay đổi trạng thái thiết bị. 
-	Replay Attack: Gửi lại các gói tin hợp lệ trước đó nhằm gây sai lệch hoạt động hệ thống. 
-	Packet Manipulation: Chỉnh sửa nội dung gói tin Modbus trong quá trình truyền. 
-	Scanning & Enumeration: Dò tìm thiết bị, địa chỉ và chức năng trong mạng Modbus. 
3.4. Đánh giá ảnh hưởng của tấn công.
-	Phân tích tác động đến hoạt động hệ thống sai lệch dữ liệu, điều khiển sai thiết bị. 
-	Đánh giá mức độ nguy hiểm và khả năng khai thác thực tế. 
-	Ghi nhận kết quả thực nghiệm và so sánh giữa các kịch bản tấn công. 
3.5. Nghiên cứu và đề xuất giải pháp bảo mật:
-	Network Segmentation: Phân tách mạng IT và OT để hạn chế truy cập trái phép. 
-	Firewall & IDS/IPS: Giám sát và phát hiện lưu lượng bất thường trong mạng Modbus. 
-	Authentication & Access Control: Kiểm soát quyền truy cập thiết bị. 
-	Secure Modbus TLS, VPN: Áp dụng mã hóa để bảo vệ dữ liệu truyền. 
-	Monitoring & Logging: Theo dõi và phát hiện sớm các hành vi tấn công. 

3.6. HSZR IOT: Firmware ESP32-S3
3.6.1. Cấu hình PlatformIO và lựa chọn thư viện
File platformio.ini định nghĩa môi trường build cho ESP32-S3:
[env:hszr_iot]
platform  = espressif32
board     = heltec_wifi_kit_32_V3
framework = arduino
monitor_speed   = 115200
monitor_filters = esp32_exception_decoder
lib_deps =
    thingpulse/ESP8266 and ESP32 OLED driver for SSD1306 displays @ ^4.4.1
build_flags =
    -DARDUINO_USB_MODE=1
    -DARDUINO_USB_CDC_ON_BOOT=1
    '-DHSZR_VERSION="1.0.0"'

ThingPulse SSD1306 được chọn thay vì thư viện Heltec chính thức (ropg/heltec_esp32_lora_v3) vì thư viện Heltec có symbol conflict với eModbus cụ thể là redeclaration của GPIO_NUM_x từ ESPIDF. ThingPulse không có xung đột này. Flag esp32_exception_decoder trong monitor_filters tự động decode stack trace khi có crash, rất hữu ích khi debug sự cố FreeRTOS.
Build flag HSZR_VERSION được inject vào firmware để hiển thị trên OLED và serial log. Macro này giúp xác định version khi troubleshoot mà không cần kết nối USB để check build date.
3.6.2. Logic phát hiện tại tầng thiết bị
Mỗi khi nhận FC06 hoặc FC16, firmware kiểm tra IP nguồn của TCP connection và so sánh với whitelist (chỉ 192.168.10.10):
// Simplified firmware logic (pseudocode)
void onModbusWriteRequest(IPAddress remoteIP, uint8_t fc,
                           uint16_t addr, uint16_t* values) {
    if (remoteIP != SCADA_AUTHORIZED_IP) {
        attackCount++;
        lastAttackerIP = remoteIP;
        Serial.printf("[ATTACK] FC%d from %s: reg=%d\n",
                      fc, remoteIP.toString().c_str(), addr);
        oledFlashAlert(remoteIP);  // nhấp nháy OLED dòng cuối
        // Vẫn thực thi (chế độ nghiên cứu)
    }
    // Cập nhật register
    xSemaphoreTake(xMutex, portMAX_DELAY);
    holdingRegs[addr - 1] = values[0];
    xSemaphoreGive(xMutex);
}
Mutex xSemaphoreTake/Give bảo vệ truy cập vào holdingRegs đây là fix cho race condition giữa OLED update task và Modbus handler task. Bỏ qua mutex là nguyên nhân crash Guru Meditation Error sau 20-30 phút chạy.
3.6.3. OLED display logic
Màn hình 128×64 pixel được chia làm 5 vùng thông tin:
Dòng 0 (y=0):  "HSZR-IOT v1.0.0"
Dòng 1 (y=10): "IP: 192.168.10.120"
Dòng 2 (y=22): "PMP:OFF VLV:CL  ALM:NO"
Dòng 3 (y=34): "T:24.5C  P:101.3kPa"
Dòng 4 (y=46): "[Polls:142  Writes:0]"   
Dòng 4 (y=46): "!! INJECTED 192.168.10.66" 
Dòng 4 toggle giữa trạng thái bình thường và alert mỗi 500ms khi đang trong trạng thái injection. Hiệu ứng nhấp nháy giúp người quan sát nhận ra ngay cả khi không nhìn thẳng vào màn hình. Update cycle toàn màn hình là 500ms, đủ mượt mà mà không tốn quá nhiều CPU.
3.6.4. Sự cố và bài học trong quá trình phát triển firmware
Sự cố 1 - Symbol conflict thư viện: Thư viện Heltec chính thức xung đột với eModbus. Mất ~4 giờ debug trước khi xác định được nguyên nhân. Giải pháp: dùng ThingPulse SSD1306 và khởi tạo OLED thủ công qua Wire.begin(SDA_PIN=17, SCL_PIN=18) theo schematic Heltec V3.
Sự cố 2 - Race condition FreeRTOS: Firmware crash với Guru Meditation Error sau 20-30 phút. esp32_exception_decoder decode stack trace chỉ vào hàm OLED update. Nguyên nhân: hai task truy cập holdingRegs không có mutex. Fix: thêm SemaphoreHandle_t, dùng xSemaphoreTake/Give bảo vệ mọi đọc/ghi buffer.
Sự cố 3 - WiFi reconnect: ESP32 thỉnh thoảng mất WiFi, nhất là khi nhiều Modbus connection đồng thời. Thêm watchdog task kiểm tra WiFi.status() mỗi 30 giây và gọi WiFi.reconnect() nếu disconnect. Downtime ~5-10 giây khi reconnect nhưng hệ thống tự phục hồi.
Sự cố 4 - Zero-based vs one-based register address: Modbus protocol dùng zero-based (register 1 = addr 0x0000 trong packet). Nhưng HSZR MBI config.json dùng 1-based ("register": 1 = PUMP). Phải thêm offset -1 trong firmware. Bug này rất confusing: write register 1 suy ra register 2 thay đổi.
3.7. HSZR SCADA: Python Control Center
3.7.1. Kiến trúc async và dependency
HSZR SCADA phải làm ba việc đồng thời mà không blocking nhau: poll Modbus mỗi 2 giây, serve HTTP/WebSocket, và theo dõi eve.json. Python asyncio với FastAPI là lựa chọn phù hợp.

Pakage	Version	Chức năng cụ thể trong HSZR
pymodbus	3.6.9	Modbus TCP async client — kết nối ESP32, gửi FC03/FC06
fastapi	>=0.110.0	REST API framework + WebSocket endpoint /ws
uvicorn[standard]	>=0.29.0	ASGI server cho FastAPI, hỗ trợ WebSocket natively
pyyaml	>=6.0	Parse file config.yaml — đọc IP, port, interval, log path
websockets	>=12.0	WebSocket protocol implementation cho push updates
Bảng 3. 2. Dependency HSZR SCADA từ requirements.txt với giải thích cụ thể
Pymodbus 3.6.9 được pin cố định vì API thay đổi lớn giữa các minor version. Phiên bản 3.x đã rewrite hoàn toàn sang async pattern, không tương thích ngược với 2.x. Nếu dùng version khác sẽ bị lỗi import hoặc method không tồn tại nhiều tutorial cũ trên internet dùng pymodbus 2.x sẽ fail.
3.7.2. Cấu hình config.yaml
scada:
  host: "192.168.10.10"
  poll_interval: 2          # giây — đọc từ ESP32 mỗi 2s
  iot_address: "192.168.10.20"
  iot_port: 502
  unit_id: 1
api:
  host: "0.0.0.0"          # bind tất cả interfaces
  port: 8000
suricata:
  eve_log: "/var/log/suricata/eve.json"
api.host: "0.0.0.0" cho phép truy cập Dashboard từ Windows host của VMware qua http://192.168.10.10:8000 mà không cần cấu hình thêm. Đây là convenience cho lab environment. poll_interval: 2 là trade off giữa data freshness và network load + CPU ESP32. Hai giây phù hợp cho loại biến trạng thái trong đồ án.
3.7.3. Pipeline tích hợp Suricata Dashboard
Đây là phần kỹ thuật hay nhất của HSZR SCADA. Luồng hoàn chỉnh:
1. Hệ thống mô phỏng phát hiện packet bất thường -> ghi alert vào /var/log/suricata/eve.json 
2. SCADA coroutine theo dõi file eve.json bằng asyncio + inotify: detect sự kiện IN_MODIFY, đọc dòng mới từ offset cuối file, parse JSON, filter event_type == "alert"
3. Alert object được enqueue vào asyncio.Queue
4. WebSocket broadcaster pop từ queue, serialize thành JSON với type: "alert", broadcast đến tất cả WebSocket client đang kết nối
5. Dashboard JavaScript nhận message, gọi addAlertItem() để tạo và insert div alert mới vào Alert Panel với animation slidein
Độ trễ tổng từ Suricata ghi alert đến Dashboard hiển thị: trong thực nghiệm thường là 800ms-1.5s. Phần lớn là thời gian Python coroutine wake up và xử lý, không phải network latency.
3.8. HSZR MBI: Rust Modbus Injector
3.8.1. Cargo.toml và release profile
[package]
name        = "hszr-mbi"
version     = "1.0.0"
edition     = "2021"
description = "HOSHIZORA Modbus Injector — OT Network Attack Simulator (Educational)"
authors     = ["Hoshizora Security Research Lab"]
[[bin]]
name = "hszr-mbi"
path = "src/main.rs"
[dependencies]
serde      = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
[profile.release]
opt-level = "z"   # tối ưu kích thước binary
lto       = true   # link-time optimization, loại unused code
strip     = true   # bỏ debug symbols khỏi binary
Chỉ hai dependency: serde + serde_json để deserialize config.json. Toàn bộ networking và Modbus framing implement từ Rust stdlib (std::net::TcpStream). Điều này giúp binary output cực nhỏ và không cần cài runtime. opt-level="z" tối ưu kích thước thay vì tốc độ hợp lý vì bottleneck là latency mạng, không phải CPU.
3.8.2. File config.json attack configuration
{
  "target":   "192.168.10.20",
  "port":     502,
  "unit_id":  1,
  "delay_ms": 600,
  "report_output": "./reports/",
  "attacks": [
    { "name": "Pump Override ON",       "fc": 6,  "register": 1, "value": 1    },
    { "name": "Valve Force Open",       "fc": 6,  "register": 2, "value": 1    },
    { "name": "Alarm Suppress",         "fc": 6,  "register": 3, "value": 0    },
    { "name": "Temperature Spoof",      "fc": 6,  "register": 4, "value": 9999 },
    { "name": "Pressure Spoof",         "fc": 6,  "register": 5, "value": 9999 },
    { "name": "Multi-write Full Chaos", "fc": 16,
      "start_reg": 1, "values": [1, 1, 0, 9999, 9999] }
  ]
}

delay_ms: 600 là thời gian chờ giữa các step trong inject. Đủ chậm để Suricata detect từng attack riêng lẻ, đủ nhanh để thể hiện tốc độ tấn công. Thiết kế JSON-based cho phép thêm attack scenario mới hoặc thay đổi target/value mà không cần recompile.
3.8.3. Implement Modbus TCP trong Rust
FC06 packet builder (pseudocode thực tế implement theo stdlib std::io::Write):
fn send_fc06(stream: &mut TcpStream, unit: u8, reg: u16, val: u16)
    -> Result<bool> {
    let txid: u16 = rand_u16();
    let packet: Vec<u8> = vec![
        (txid >> 8) as u8, (txid & 0xFF) as u8, // Transaction ID
        0x00, 0x00,                              // Protocol ID
        0x00, 0x06,                              // Length = 6
        unit,                                    // Unit ID
        0x06,                                    // Function Code
        (reg >> 8) as u8, (reg & 0xFF) as u8,   // Register (1-based)
        (val >> 8) as u8, (val & 0xFF) as u8,   // Value
    ];
    stream.write_all(&packet)?;
    // Đọc response (12 bytes echo)
    let mut buf = [0u8; 12];
    stream.read_exact(&mut buf)?;
    // Verify echo: response phải giống request
    Ok(buf[7..] == packet[7..])
}

FC16 phức tạp hơn: phần Data gồm count (2 byte), byte_count (1 byte) và values[]. Tổng packet size: 7 (MBAP) + 1 (FC) + 2 (start_reg) + 2 (count) + 1 (byte_count) + count×2 (values) bytes. Verify response: FC16 response chỉ echo start_reg + count, không echo toàn bộ values.
3.9. Suricata IDS/IPS Gateway
3.9.1. Setup script phân tích từng bước
#!/usr/bin/env bash
set -euo pipefail
IFACE_A="${1:-eth1}"   # NIC phía MBI + SCADA
IFACE_B="${2:-eth2}"   # NIC phía ESP32
BRIDGE="br0"
# Bước 1: Tạo bridge
ip link add name br0 type bridge 2>/dev/null || true
ip link set $IFACE_A master br0
ip link set $IFACE_B master br0
ip link set $IFACE_A up
ip link set $IFACE_B up
ip link set br0 up
# Bước 2: IP forwarding + tắt iptables bridge filtering (CRITICAL)
sysctl -w net.ipv4.ip_forward=1 > /dev/null
sysctl -w net.bridge.bridge-nf-call-iptables=0
sysctl -w net.bridge.bridge-nf-call-ip6tables=0
# Bước 3: Copy rules và config
cp modbus.rules /etc/suricata/rules/modbus.rules
cp suricata.yaml /etc/suricata/hoshizora.yaml
# Bước 4: Validate config
suricata -T -c /etc/suricata/hoshizora.yaml --af-packet=br0
# Bước 5: Start daemon
suricata -c /etc/suricata/hoshizora.yaml --af-packet=br0 -D \
    --pidfile /var/run/suricata.pid

Hai dòng sysctl bridge nf call iptables=0 là critical và thường bị bỏ sót. Nếu không tắt, iptables sẽ xử lý traffic qua bridge theo default rules và có thể drop packet trước khi Suricata nhận được gây ra tình trạng bridge hoạt động nhưng Suricata không thấy gì. Đây chính xác là bug mà nhóm lab mất hàng giờ debug.
3.9.2. suricata.yaml và cấu hình chi tiết
vars:
  address-groups:
    HOME_NET:     "[192.168.10.0/24]"
    SCADA_SERVER: "[192.168.10.10]"
    MODBUS_SLAVE: "[192.168.10.120]"
  port-groups:
    MODBUS_PORTS: "502"
af-packet:
  - interface: br0
    cluster-id: 99
    cluster-type: cluster_flow   # tất cả packet cùng flow → cùng thread
    defrag: yes
    # copy-mode: ips            # uncomment để bật IPS mode
outputs:
  - eve-log:
      enabled: yes
      filename: /var/log/suricata/eve.json
      types:
        - alert:
            payload: yes         # base64 payload để forensics
            payload-printable: yes
            packet: yes

cluster_type: cluster_flow đảm bảo TCP reassembly hoạt động đúng tất cả packet cùng một TCP flow xử lý bởi cùng một worker thread, cho phép Suricata track TCP session và parse Modbus application layer đúng cách. Nếu dùng cluster_cpu, packet cùng flow có thể bị split sang thread khác, gây lỗi reassembly.
3.10. HOSH Dashboard
3.10.1. Layout và cấu trúc component
Dashboard dùng CSS Grid với hai cột, ba hàng:
.main-grid {
    display: grid;
    grid-template-columns: 1fr 340px;  /* content và sidebar */
    grid-template-rows: auto auto 1fr;
    gap: 18px;
    padding: 20px 24px;
}
/* Đặc biệt: Log panel span 2 hàng để có chiều cao đủ */
#log-panel   { grid-column: 2; grid-row: 2 / 4; }
#alert-panel { grid-column: 1; }
Năm Register Cards chiếm full width của row 1 (grid-column: 1 / -1). Network Topology SVG và Attack Controls nằm ở column 1. SCADA Poll Log span hai hàng ở sidebar. Alert Panel ở dưới Topology. Cấu trúc này cho phép operator nhìn thấy cả dữ liệu real-time lẫn lịch sử log và alert cùng lúc mà không cần scroll.
3.10.2. Design tokens
:root {
  --bg-base:    #070d1a;                    /* nền tối navy */
  --bg-panel:   rgba(10, 22, 45, 0.75);     /* glassmorphism panel */
  --border:     rgba(0, 200, 170, 0.15);    /* viền teal mờ */
  --accent:     #00c8aa;                    /* màu teal chủ đạo */
  --danger:     #ff3f3f;                    /* alert đỏ */
  --warn:       #ffb830;                    /* warning vàng */
  --ok:         #00e676;                    /* trạng thái tốt */
  --text-mono:  'JetBrains Mono', monospace; /* font monospace */
}
Màu #070d1a (Màu navy đậm gần như black) cộng radial gradient ở góc tạo aesthetic "industrial HUD"  tham khảo từ screenshot Wonderware InTouch và FactoryTalk. Glassmorphism panels với backdrop-filter: blur(14px) tạo chiều sâu. Register cards thay đổi màu border theo trạng thái (xanh lá = normal, đỏ = anomaly, vàng nhấp nháy = alarm active).
3.10.3. Network Topology SVG inline
Sơ đồ mạng được vẽ bằng SVG inline trong index.html không cần thư viện đồ thị ngoài như D3 hay Cytoscape:
<svg viewBox="0 0 580 200" width="100%">
  <!-- SCADA node (teal) -->
  <rect x="10" y="70" width="130" height="60" rx="8"
        fill="rgba(0,200,170,0.12)" stroke="#00c8aa" stroke-width="1.5"/>
  <!-- Arrow SCADA → Suricata (authorized) -->
  <line x1="140" y1="100" x2="220" y2="100"
        stroke="#00c8aa" stroke-width="2" stroke-dasharray="6 3"/>
  <!-- MBI node (red, attacker) -->
  <rect x="220" y="4" width="140" height="48" rx="8"
        fill="rgba(255,63,63,0.12)" stroke="#ff3f3f" stroke-width="1.5"/>
  <!-- Arrow MBI → bridge (attack) -->
  <line x1="290" y1="52" x2="290" y2="70"
        stroke="#ff3f3f" stroke-width="2" stroke-dasharray="4 3"/>
</svg>

SVG inline render ngay lập tức, scale tốt trên mọi viewport, không cần load thêm script. Màu sắc dùng CSS variables của design system. Đây là lựa chọn đơn giản nhưng hiệu quả cho một research dashboard.
 
