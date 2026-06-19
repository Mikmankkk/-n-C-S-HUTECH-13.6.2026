#!/usr/bin/env bash
# ============================================================
# Hoshizora — Suricata IDS/IPS Setup Script
# Run as root on the gateway VM (Debian/Ubuntu)
# ============================================================

set -euo pipefail

RULES_SRC="$(dirname "$0")/rules/modbus.rules"
RULES_DST="/etc/suricata/rules/modbus.rules"
CONF_SRC="$(dirname "$0")/suricata.yaml"
CONF_DST="/etc/suricata/hoshizora.yaml"
IFACE_A="${1:-eth1}"   # NIC facing HSZR-MBI
IFACE_B="${2:-eth2}"   # NIC facing HSZR-IOT
BRIDGE="br0"

echo "══════════════════════════════════════════"
echo "  HOSHIZORA Suricata Setup"
echo "  Bridge:  $BRIDGE  ($IFACE_A <-> $IFACE_B)"
echo "══════════════════════════════════════════"

# ── 1. Install Suricata ───────────────────────────────────────────────────────
if ! command -v suricata &>/dev/null; then
    echo "[*] Installing Suricata..."
    apt-get update -q
    apt-get install -y suricata
fi
echo "[✓] Suricata: $(suricata --build-info | grep 'Version' | head -1)"

# ── 2. Create bridge interface ────────────────────────────────────────────────
echo "[*] Creating bridge $BRIDGE..."
ip link add name "$BRIDGE" type bridge 2>/dev/null || true
ip link set "$IFACE_A" master "$BRIDGE"
ip link set "$IFACE_B" master "$BRIDGE"
ip link set "$IFACE_A" up
ip link set "$IFACE_B" up
ip link set "$BRIDGE" up
echo "[✓] Bridge $BRIDGE UP"

# ── 3. Enable IP forwarding ───────────────────────────────────────────────────
sysctl -w net.ipv4.ip_forward=1 > /dev/null
echo "[✓] IP forwarding enabled"

# ── 4. Install rules ──────────────────────────────────────────────────────────
mkdir -p /etc/suricata/rules
cp "$RULES_SRC" "$RULES_DST"
echo "[✓] Rules installed → $RULES_DST"

# ── 5. Install config ─────────────────────────────────────────────────────────
cp "$CONF_SRC" "$CONF_DST"
echo "[✓] Config installed → $CONF_DST"

# ── 6. Test config ────────────────────────────────────────────────────────────
echo "[*] Validating config..."
suricata -T -c "$CONF_DST" --af-packet="$BRIDGE" && echo "[✓] Config OK"

# ── 7. Start Suricata ─────────────────────────────────────────────────────────
echo "[*] Starting Suricata in IDS mode on $BRIDGE..."
suricata -c "$CONF_DST" --af-packet="$BRIDGE" -D \
  --pidfile /var/run/suricata.pid

echo ""
echo "✓ Suricata running. Tail alerts with:"
echo "    tail -f /var/log/suricata/eve.json | jq 'select(.event_type==\"alert\")'"
echo ""
echo "  To enable IPS drop mode, add to suricata.yaml:"
echo "    af-packet: copy-mode: ips"
