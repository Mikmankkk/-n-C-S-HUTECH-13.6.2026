# HSZR-IOT — ESP32-S3 Heltec Firmware

Modbus TCP Slave firmware for the Hoshizora ICS Security Research Platform.

## Hardware

- **Board:** Heltec ESP32-S3 V3 (128×64 OLED)
- **IP:** `192.168.10.20` (static)
- **Modbus port:** `502`
- **Unit ID:** `1`

## Register Map

| Address | Name           | Values               |
|---------|----------------|----------------------|
| 0x0001  | PUMP_CONTROL   | 0=Off, 1=On          |
| 0x0002  | VALVE_STATUS   | 0=Closed, 1=Open     |
| 0x0003  | ALARM_OVERRIDE | 0=Normal, 1=Alarm    |
| 0x0004  | TEMPERATURE    | °C × 10 (e.g. 250 = 25.0°C) |
| 0x0005  | PRESSURE       | kPa × 10             |

## OLED Display

```
HSZR-IOT v1.0.0
IP: 192.168.10.20
PMP:OFF VLV:CL  ALM:NO
T:25.0C  P:101.3kPa
[Polls:42 Writes:0]
```

When written to from a **non-SCADA IP**, the bottom row flashes:
```
!! INJECTED 192.168.10.66
```

## Setup

1. Open project in **PlatformIO** (VSCode extension or CLI).
2. Edit `src/main.cpp`:
   - Set `WIFI_SSID` and `WIFI_PASS`.
   - Verify `STATIC_IP`, `GATEWAY`, `SUBNET`.
3. Flash: `pio run -t upload`

## Board Version Note

- **V3 (default):** uses `heltec_unofficial.h`
- **V2:** replace `#include "heltec_unofficial.h"` with `#include "heltec.h"`
  and update `Heltec.begin(...)` call per V2 library documentation.
