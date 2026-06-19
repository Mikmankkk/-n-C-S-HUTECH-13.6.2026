"""
HOSHIZORA SCADA — Suricata EVE JSON Alert Reader
Tails /var/log/suricata/eve.json for Modbus-related alerts (port 502).
"""
import json
import logging
import threading
from collections import deque
from pathlib import Path
from time import sleep

logger = logging.getLogger("hszr.scada.alerts")


class AlertReader:
    def __init__(self, eve_log_path: str = "/var/log/suricata/eve.json", max_alerts: int = 500):
        self.eve_log_path = Path(eve_log_path)
        self._alerts: deque = deque(maxlen=max_alerts)
        self._new_queue: list = []
        self._lock = threading.Lock()
        self._running = False
        self._thread: threading.Thread | None = None

    def start(self):
        self._running = True
        self._thread = threading.Thread(target=self._tail_loop, daemon=True, name="alert-reader")
        self._thread.start()
        logger.info(f"Alert reader watching: {self.eve_log_path}")

    def stop(self):
        self._running = False

    # ---------------------------------------------------------------- internal
    def _tail_loop(self):
        # Wait for file to appear
        while self._running and not self.eve_log_path.exists():
            logger.warning(f"Waiting for eve.json at {self.eve_log_path} ...")
            sleep(3)

        try:
            with open(self.eve_log_path, "r", encoding="utf-8", errors="replace") as f:
                f.seek(0, 2)  # seek to end
                while self._running:
                    line = f.readline()
                    if not line:
                        sleep(0.3)
                        continue
                    self._parse_line(line)
        except Exception as e:
            logger.error(f"Alert reader crashed: {e}")

    def _parse_line(self, line: str):
        try:
            event = json.loads(line.strip())
        except json.JSONDecodeError:
            return

        if event.get("event_type") != "alert":
            return

        # Only care about Modbus port 502
        if event.get("dest_port") != 502:
            return

        alert_sig = event.get("alert", {}).get("signature", "Unknown")
        src_ip = event.get("src_ip", "?")
        logger.warning(f"[SURICATA ALERT] {alert_sig} from {src_ip}")

        with self._lock:
            self._alerts.append(event)
            self._new_queue.append(event)

    # ---------------------------------------------------------------- public
    def drain_new(self) -> list:
        """Return and clear newly received alerts since last call."""
        with self._lock:
            batch = list(self._new_queue)
            self._new_queue.clear()
        return batch

    def recent_alerts(self, limit: int = 50) -> list:
        with self._lock:
            return list(self._alerts)[-limit:]
