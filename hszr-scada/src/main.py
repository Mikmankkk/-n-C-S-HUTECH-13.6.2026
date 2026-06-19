"""
HOSHIZORA SCADA — Main Entry Point
Runs the Modbus polling loop and FastAPI server concurrently.
"""
import asyncio
import logging
import os
import sys
from datetime import datetime

import uvicorn

sys.path.insert(0, os.path.dirname(__file__))

from config import load_config
from modbus_master import ModbusMaster
from alert_reader import AlertReader
from api import app, manager, init_api

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s  [%(levelname)-8s]  %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("hszr.scada")

BANNER = r"""
╔══════════════════════════════════════════════════╗
║   ██╗  ██╗ ██████╗ ███████╗██╗  ██╗             ║
║   ██║  ██║██╔═══██╗██╔════╝██║  ██║             ║
║   ███████║██║   ██║███████╗███████║             ║
║   ██╔══██║██║   ██║╚════██║██╔══██║             ║
║   ██║  ██║╚██████╔╝███████║██║  ██║             ║
║   ╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚═╝  ╚═╝             ║
║                                                  ║
║   HSZR-SCADA  |  ICS Security Research Platform  ║
║   Modbus TCP Master + IDS Dashboard              ║
╚══════════════════════════════════════════════════╝
"""


async def polling_loop(master: ModbusMaster, alert_reader: AlertReader, interval: float):
    logger.info(f"Polling loop started — interval: {interval}s")
    while True:
        try:
            registers = await master.poll()
            if registers:
                await manager.broadcast({
                    "type": "register_update",
                    "data": {
                        "timestamp": datetime.now().isoformat(),
                        "registers": registers,
                    },
                })

            # Forward any new Suricata alerts to dashboard
            for alert in alert_reader.drain_new():
                await manager.broadcast({"type": "alert", "data": alert})

            # Broadcast latest log line
            if master.recent_logs:
                await manager.broadcast({
                    "type": "scada_log",
                    "data": master.recent_logs[-1],
                })

        except Exception as e:
            logger.error(f"Polling error: {e}")

        await asyncio.sleep(interval)


async def main():
    print(BANNER)

    cfg_path = os.environ.get("HSZR_CONFIG", "config.yaml")
    cfg = load_config(cfg_path)

    logger.info(f"IoT target : {cfg.scada.iot_address}:{cfg.scada.iot_port}  unit={cfg.scada.unit_id}")
    logger.info(f"API server : http://{cfg.api.host}:{cfg.api.port}")
    logger.info(f"Dashboard  : http://{cfg.api.host}:{cfg.api.port}/")
    logger.info(f"Eve log    : {cfg.suricata.eve_log}")

    master = ModbusMaster(
        host=cfg.scada.iot_address,
        port=cfg.scada.iot_port,
        unit_id=cfg.scada.unit_id,
    )

    alert_reader = AlertReader(eve_log_path=cfg.suricata.eve_log)
    alert_reader.start()

    init_api(master, alert_reader)

    server = uvicorn.Server(uvicorn.Config(
        app=app,
        host=cfg.api.host,
        port=cfg.api.port,
        log_level="warning",
    ))

    await asyncio.gather(
        server.serve(),
        polling_loop(master, alert_reader, cfg.scada.poll_interval),
    )


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        logger.info("Shutdown")
