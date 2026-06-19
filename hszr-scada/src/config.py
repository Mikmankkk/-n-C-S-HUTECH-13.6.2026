"""
HOSHIZORA SCADA — Configuration Loader
Reads config.yaml; falls back to safe defaults if missing.
"""
import os
import yaml
from dataclasses import dataclass, field


@dataclass
class ScadaConfig:
    host: str = "192.168.10.10"
    poll_interval: float = 2.0
    iot_address: str = "192.168.10.20"
    iot_port: int = 502
    unit_id: int = 1


@dataclass
class ApiConfig:
    host: str = "0.0.0.0"
    port: int = 8000


@dataclass
class SuricataConfig:
    eve_log: str = "/var/log/suricata/eve.json"


@dataclass
class AppConfig:
    scada: ScadaConfig = field(default_factory=ScadaConfig)
    api: ApiConfig = field(default_factory=ApiConfig)
    suricata: SuricataConfig = field(default_factory=SuricataConfig)


_CONFIG: AppConfig | None = None


def load_config(path: str = "config.yaml") -> AppConfig:
    global _CONFIG
    defaults = AppConfig()

    if not os.path.exists(path):
        _CONFIG = defaults
        return _CONFIG

    with open(path, "r") as f:
        raw = yaml.safe_load(f) or {}

    s = raw.get("scada", {})
    a = raw.get("api", {})
    su = raw.get("suricata", {})

    _CONFIG = AppConfig(
        scada=ScadaConfig(
            host=s.get("host", defaults.scada.host),
            poll_interval=s.get("poll_interval", defaults.scada.poll_interval),
            iot_address=s.get("iot_address", defaults.scada.iot_address),
            iot_port=s.get("iot_port", defaults.scada.iot_port),
            unit_id=s.get("unit_id", defaults.scada.unit_id),
        ),
        api=ApiConfig(
            host=a.get("host", defaults.api.host),
            port=a.get("port", defaults.api.port),
        ),
        suricata=SuricataConfig(
            eve_log=su.get("eve_log", defaults.suricata.eve_log),
        ),
    )
    return _CONFIG


def get_config() -> AppConfig:
    global _CONFIG
    if _CONFIG is None:
        _CONFIG = load_config()
    return _CONFIG
