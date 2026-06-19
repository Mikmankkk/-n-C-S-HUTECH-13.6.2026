/*
 * HOSHIZORA — HSZR-IOT Firmware
 * Target  : ESP32-S3 Heltec WiFi Kit 32 V3 (128×64 SSD1306 OLED)
 * Library : ThingPulse/ESP8266 and ESP32 OLED driver for SSD1306 displays
 *
 * Role    : Modbus TCP Slave — port 502, unit ID 1.
 *           Implements FC03 (read holding registers),
 *                      FC06 (write single register),
 *                      FC16 (write multiple registers).
 *           OLED shows live register states; flashes !! INJECTED when
 *           written to from a non-SCADA IP address.
 *
 * ── Register Map ──────────────────────────────────────────────
 *   Addr 1  PUMP_CONTROL    0=Off,  1=On
 *   Addr 2  VALVE_STATUS    0=Closed, 1=Open
 *   Addr 3  ALARM_OVERRIDE  0=Normal, 1=Alarm
 *   Addr 4  TEMPERATURE     Simulated °C × 10  (e.g. 250 = 25.0°C)
 *   Addr 5  PRESSURE        Simulated kPa × 10 (e.g. 1013 = 101.3 kPa)
 *
 * ── Heltec WiFi Kit 32 V3 OLED pins ──────────────────────────
 *   SDA = 17  |  SCL = 18  |  RST = 21
 *   OLED address: 0x3C
 */

#include <Wire.h>
#include <WiFi.h>
#include "SSD1306Wire.h"   // ThingPulse driver

// ─── OLED ─────────────────────────────────────────────────────────────────────
// Heltec WiFi Kit 32 V3: SDA=17, SCL=18, RST=21, I2C addr=0x3C, 128×64
#define OLED_SDA  17
#define OLED_SCL  18
#define OLED_RST  21
#define OLED_ADDR 0x3C

SSD1306Wire oled(OLED_ADDR, OLED_SDA, OLED_SCL);

// ─── Network ──────────────────────────────────────────────────────────────────
const char* WIFI_SSID  = "YOUR_SSID";   // ← change
const char* WIFI_PASS  = "YOUR_PASS";   // ← change

IPAddress STATIC_IP(192, 168, 10, 20);
IPAddress GATEWAY  (192, 168, 10,  1);
IPAddress SUBNET   (255, 255, 255,  0);
IPAddress DNS1     (  8,   8,   8,  8);

const uint16_t MODBUS_PORT = 502;
const uint8_t  UNIT_ID     = 1;
const uint16_t REG_COUNT   = 5;   // addresses 1–5

// ─── Holding register store (0-indexed: regs[0] = addr 1) ────────────────────
uint16_t regs[REG_COUNT] = { 0, 0, 0, 250, 1013 };

// ─── Authorised master ────────────────────────────────────────────────────────
const IPAddress SCADA_IP(192, 168, 10, 10);

// ─── Injection tracking ───────────────────────────────────────────────────────
char       lastWriterStr[20] = "";
bool       wasInjected  = false;
uint32_t   injectedAt   = 0;
uint32_t   pollCount    = 0;
uint32_t   writeCount   = 0;

WiFiServer modbusServer(MODBUS_PORT);

// ─── Modbus helpers ───────────────────────────────────────────────────────────
inline uint16_t u16be(const uint8_t* b)         { return ((uint16_t)b[0] << 8) | b[1]; }
inline void     put16be(uint8_t* b, uint16_t v) { b[0] = v >> 8; b[1] = v & 0xFF; }

// ─── OLED ─────────────────────────────────────────────────────────────────────
void updateDisplay() {
    oled.clear();
    oled.setFont(ArialMT_Plain_10);

    oled.drawString(0,  0, "HSZR-IOT  " HSZR_VERSION);

    if (WiFi.status() == WL_CONNECTED)
        oled.drawString(0, 12, "IP: " + WiFi.localIP().toString());
    else
        oled.drawString(0, 12, "WiFi connecting...");

    char row2[34];
    snprintf(row2, sizeof(row2), "PMP:%s VLV:%s ALM:%s",
             regs[0] ? "ON " : "OFF",
             regs[1] ? "OP " : "CL ",
             regs[2] ? "YES" : "NO ");
    oled.drawString(0, 23, row2);

    char row3[34];
    snprintf(row3, sizeof(row3), "T:%.1fC P:%.1fkPa",
             regs[3] / 10.0f, regs[4] / 10.0f);
    oled.drawString(0, 35, row3);

    if (wasInjected && (millis() - injectedAt) < 8000) {
        char row4[34];
        snprintf(row4, sizeof(row4), "!!INJECTED %s", lastWriterStr);
        oled.drawString(0, 48, row4);
    } else {
        wasInjected = false;
        char row4[34];
        snprintf(row4, sizeof(row4), "P:%lu W:%lu", pollCount, writeCount);
        oled.drawString(0, 48, row4);
    }

    oled.display();
}

// ─── Modbus TCP request handler ───────────────────────────────────────────────
void handleModbusClient(WiFiClient& client) {
    uint8_t buf[260] = {};

    // Wait for MBAP header (7 bytes)
    uint32_t t0 = millis();
    while (client.available() < 7 && millis() - t0 < 500) delay(1);
    if (client.available() < 7) return;

    client.readBytes(buf, 7);
    uint16_t txID   = u16be(buf + 0);
    uint16_t protID = u16be(buf + 2);
    uint16_t pduLen = u16be(buf + 4) - 1;   // length field includes unit_id byte
    uint8_t  unitID = buf[6];

    if (protID != 0)                          return;
    if (unitID != UNIT_ID && unitID != 0xFF)  return;
    if (pduLen == 0 || pduLen > 249)          return;

    t0 = millis();
    while (client.available() < (int)pduLen && millis() - t0 < 500) delay(1);
    if (client.available() < (int)pduLen) return;
    client.readBytes(buf + 7, pduLen);

    uint8_t  fc         = buf[7];
    IPAddress clientIP  = client.remoteIP();
    bool      authorised = (clientIP == SCADA_IP);

    // ── FC03: Read Holding Registers ──────────────────────────────────────────
    if (fc == 0x03) {
        uint16_t startReg = u16be(buf + 8);
        uint16_t qty      = u16be(buf + 10);
        pollCount++;

        if (startReg < 1 || qty == 0 || startReg - 1 + qty > REG_COUNT) {
            uint8_t ex[9] = { buf[0], buf[1], 0, 0, 0, 3, unitID, 0x83, 0x02 };
            client.write(ex, 9);
            return;
        }
        uint8_t byteCount = qty * 2;
        uint8_t resp[260] = {};
        put16be(resp + 0, txID);
        put16be(resp + 2, 0);
        put16be(resp + 4, 3 + byteCount);
        resp[6] = unitID;
        resp[7] = 0x03;
        resp[8] = byteCount;
        for (uint16_t i = 0; i < qty; i++)
            put16be(resp + 9 + i * 2, regs[startReg - 1 + i]);
        client.write(resp, 9 + byteCount);
        return;
    }

    // ── FC06: Write Single Register ───────────────────────────────────────────
    if (fc == 0x06) {
        uint16_t regAddr = u16be(buf + 8);
        uint16_t value   = u16be(buf + 10);

        if (regAddr < 1 || regAddr > REG_COUNT) {
            uint8_t ex[9] = { buf[0], buf[1], 0, 0, 0, 3, unitID, 0x86, 0x02 };
            client.write(ex, 9);
            return;
        }
        regs[regAddr - 1] = value;
        writeCount++;

        if (!authorised) {
            snprintf(lastWriterStr, sizeof(lastWriterStr), "%s", clientIP.toString().c_str());
            wasInjected = true;
            injectedAt  = millis();
            Serial.printf("[INJECTED] FC06 reg%u=%u src=%s\n",
                          regAddr, value, lastWriterStr);
        }
        client.write(buf, 12);   // echo response
        return;
    }

    // ── FC16: Write Multiple Registers ────────────────────────────────────────
    if (fc == 0x10) {
        uint16_t startReg = u16be(buf + 8);
        uint16_t qty      = u16be(buf + 10);

        if (startReg < 1 || qty == 0 || startReg - 1 + qty > REG_COUNT) {
            uint8_t ex[9] = { buf[0], buf[1], 0, 0, 0, 3, unitID, 0x90, 0x02 };
            client.write(ex, 9);
            return;
        }
        for (uint16_t i = 0; i < qty; i++)
            regs[startReg - 1 + i] = u16be(buf + 13 + i * 2);
        writeCount++;

        if (!authorised) {
            snprintf(lastWriterStr, sizeof(lastWriterStr), "%s", clientIP.toString().c_str());
            wasInjected = true;
            injectedAt  = millis();
            Serial.printf("[INJECTED] FC16 start=%u qty=%u src=%s\n",
                          startReg, qty, lastWriterStr);
        }
        uint8_t resp[12] = {};
        put16be(resp + 0, txID);
        put16be(resp + 2, 0);
        put16be(resp + 4, 6);
        resp[6] = unitID;
        resp[7] = 0x10;
        put16be(resp + 8,  startReg);
        put16be(resp + 10, qty);
        client.write(resp, 12);
        return;
    }

    // ── Unknown FC — Exception 01 ─────────────────────────────────────────────
    uint8_t ex[9] = { buf[0], buf[1], 0, 0, 0, 3, unitID, (uint8_t)(fc | 0x80), 0x01 };
    client.write(ex, 9);
}

// ─── Setup ────────────────────────────────────────────────────────────────────
void setup() {
    Serial.begin(115200);

    // OLED reset
    pinMode(OLED_RST, OUTPUT);
    digitalWrite(OLED_RST, LOW);
    delay(50);
    digitalWrite(OLED_RST, HIGH);

    // OLED init (ThingPulse)
    Wire.begin(OLED_SDA, OLED_SCL);
    oled.init();
    oled.flipScreenVertically();
    oled.setFont(ArialMT_Plain_10);
    oled.clear();
    oled.drawString(0, 0, "HSZR-IOT Booting...");
    oled.drawString(0, 12, HSZR_VERSION);
    oled.display();

    // Static IP then connect
    WiFi.config(STATIC_IP, GATEWAY, SUBNET, DNS1);
    WiFi.begin(WIFI_SSID, WIFI_PASS);

    oled.drawString(0, 24, "Connecting WiFi...");
    oled.display();

    uint32_t wt = millis();
    while (WiFi.status() != WL_CONNECTED && millis() - wt < 15000) delay(300);

    if (WiFi.status() == WL_CONNECTED) {
        Serial.printf("[NET] IP: %s\n", WiFi.localIP().toString().c_str());
        modbusServer.begin();
        Serial.printf("[MODBUS] Port=%u  UnitID=%u\n", MODBUS_PORT, UNIT_ID);
    } else {
        Serial.println("[ERROR] WiFi connection failed — check credentials");
        oled.drawString(0, 36, "WiFi FAILED!");
        oled.display();
    }

    updateDisplay();
}

// ─── Loop ─────────────────────────────────────────────────────────────────────
void loop() {
    WiFiClient client = modbusServer.available();
    if (client) {
        Serial.printf("[CONN] %s\n", client.remoteIP().toString().c_str());
        while (client.connected()) {
            if (client.available()) {
                handleModbusClient(client);
                updateDisplay();
            }
            delay(1);
        }
        client.stop();
    }
}
