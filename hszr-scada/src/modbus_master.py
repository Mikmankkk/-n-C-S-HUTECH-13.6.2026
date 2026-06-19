"""
HOSHIZORA SCADA — Modbus TCP Master
Polls HSZR-IOT registers and issues authorized write commands.
"""
import asyncio
import logging
from datetime import datetime
from collections import deque

from pymodbus.client import AsyncModbusTcpClient
from pymodbus.exceptions import ModbusException

logger = logging.getLogger("hszr.scada.modbus")

# Shared register map (address → name)
REGISTER_MAP = {
    1: "PUMP_CONTROL",
    2: "VALVE_STATUS",
    3: "ALARM_OVERRIDE",
    4: "TEMPERATURE",
    5: "PRESSURE",
}


class ModbusMaster:
    def __init__(self, host: str, port: int, unit_id: int):
        self.host = host
        self.port = port
        self.unit_id = unit_id
        self._client: AsyncModbusTcpClient | None = None
        self._last_registers: dict = {}
        self._poll_count: int = 0
        self._log_buffer: deque = deque(maxlen=200)

    # ------------------------------------------------------------------ connect
    async def connect(self):
        self._client = AsyncModbusTcpClient(host=self.host, port=self.port)
        await self._client.connect()
        logger.info(f"Connected to HSZR-IOT at {self.host}:{self.port}")

    def close(self):
        if self._client:
            self._client.close()

    # ------------------------------------------------------------------ logging
    def _log(self, msg: str) -> dict:
        entry = {"timestamp": datetime.now().isoformat(), "message": msg}
        self._log_buffer.append(entry)
        return entry

    # ------------------------------------------------------------------ poll
    async def poll(self) -> dict:
        """FC03 — Read holding registers 1-5. Returns {name: value} dict."""
        if not self._client or not self._client.connected:
            try:
                await self.connect()
            except Exception as e:
                self._log(f"[ERROR] Connect failed: {e}")
                return {}

        try:
            result = await self._client.read_holding_registers(
                address=1, count=5, slave=self.unit_id
            )
            if result.isError():
                self._log(f"[WARN] Poll error: {result}")
                return {}

            self._poll_count += 1
            registers = {
                REGISTER_MAP[i + 1]: val
                for i, val in enumerate(result.registers)
            }
            self._last_registers = registers
            self._log(
                f"Poll #{self._poll_count}: "
                + " | ".join(f"{k}={v}" for k, v in registers.items())
            )
            return registers

        except ModbusException as e:
            self._log(f"[ERROR] Modbus: {e}")
            return {}
        except Exception as e:
            self._log(f"[ERROR] Unexpected: {e}")
            self._client = None  # force reconnect
            return {}

    # ------------------------------------------------------------------ write
    async def write_register(self, register: int, value: int) -> bool:
        """FC06 — Authorized single-register write from SCADA."""
        if not self._client or not self._client.connected:
            try:
                await self.connect()
            except Exception:
                return False
        try:
            result = await self._client.write_register(
                address=register, value=value, slave=self.unit_id
            )
            if result.isError():
                self._log(f"[WARN] Write reg{register}={value} FAILED")
                return False
            name = REGISTER_MAP.get(register, f"REG_{register}")
            self._log(f"[CMD] Write {name}={value} -> OK")
            return True
        except Exception as e:
            self._log(f"[ERROR] Write: {e}")
            return False

    # ------------------------------------------------------------------ props
    @property
    def last_registers(self) -> dict:
        return dict(self._last_registers)

    @property
    def recent_logs(self) -> list:
        return list(self._log_buffer)[-50:]
