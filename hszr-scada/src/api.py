"""
HOSHIZORA SCADA — FastAPI REST + WebSocket Server
Exposes /status, /alerts, /logs, /command, and WebSocket /ws
Also serves the dashboard as static files from ../dashboard/
"""
import asyncio
import json
import logging
from typing import List

from fastapi import FastAPI, WebSocket, WebSocketDisconnect, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel

logger = logging.getLogger("hszr.scada.api")

app = FastAPI(
    title="HOSHIZORA SCADA API",
    version="1.0.0",
    description="Hoshizora ICS/SCADA Security Research Platform",
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)


# ─── WebSocket Connection Manager ────────────────────────────────────────────

class ConnectionManager:
    def __init__(self):
        self.active: List[WebSocket] = []

    async def connect(self, ws: WebSocket):
        await ws.accept()
        self.active.append(ws)
        logger.info(f"Dashboard connected — active clients: {len(self.active)}")

    def disconnect(self, ws: WebSocket):
        if ws in self.active:
            self.active.remove(ws)

    async def broadcast(self, data: dict):
        if not self.active:
            return
        msg = json.dumps(data)
        dead = []
        for ws in self.active:
            try:
                await ws.send_text(msg)
            except Exception:
                dead.append(ws)
        for ws in dead:
            self.disconnect(ws)


manager = ConnectionManager()

# Injected by main.py after startup
_modbus_master = None
_alert_reader = None


def init_api(modbus_master, alert_reader):
    global _modbus_master, _alert_reader
    _modbus_master = modbus_master
    _alert_reader = alert_reader


# ─── HTTP Endpoints ───────────────────────────────────────────────────────────

class CommandRequest(BaseModel):
    register: int
    value: int


@app.get("/status", summary="Current register snapshot")
async def get_status():
    if _modbus_master is None:
        raise HTTPException(status_code=503, detail="SCADA not initialised")
    return {
        "registers": _modbus_master.last_registers,
        "connected": _modbus_master._client is not None,
    }


@app.get("/alerts", summary="Recent Suricata alerts")
async def get_alerts(limit: int = 50):
    if _alert_reader is None:
        return {"alerts": []}
    return {"alerts": _alert_reader.recent_alerts(limit)}


@app.get("/logs", summary="Recent SCADA poll log")
async def get_logs(limit: int = 50):
    if _modbus_master is None:
        return {"logs": []}
    return {"logs": _modbus_master.recent_logs[-limit:]}


@app.post("/command", summary="Issue authorized register write")
async def send_command(cmd: CommandRequest):
    if _modbus_master is None:
        raise HTTPException(status_code=503, detail="SCADA not initialised")
    ok = await _modbus_master.write_register(cmd.register, cmd.value)
    return {"success": ok, "register": cmd.register, "value": cmd.value}


# ─── WebSocket ────────────────────────────────────────────────────────────────

@app.websocket("/ws")
async def websocket_endpoint(ws: WebSocket):
    await manager.connect(ws)
    try:
        # Send initial snapshot on connect
        await ws.send_text(json.dumps({
            "type": "init",
            "data": {
                "registers": _modbus_master.last_registers if _modbus_master else {},
                "alerts": _alert_reader.recent_alerts(20) if _alert_reader else [],
                "logs": _modbus_master.recent_logs[-20:] if _modbus_master else [],
            },
        }))
        # Keep-alive ping loop
        while True:
            await asyncio.sleep(15)
            await ws.send_text(json.dumps({"type": "ping"}))
    except WebSocketDisconnect:
        manager.disconnect(ws)
    except Exception:
        manager.disconnect(ws)


# ─── Static dashboard ─────────────────────────────────────────────────────────
try:
    app.mount("/", StaticFiles(directory="../dashboard", html=True), name="dashboard")
except RuntimeError:
    pass  # Dashboard dir not present — API-only mode
