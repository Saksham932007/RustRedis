"""Linux machine-state snapshot helper for benchmark runs.

Captures the signals needed to explain the fast/slow throughput states
observed in earlier experiment versions: CPU frequency/governor, thermal
zone temperatures, memory/swap pressure, load average, and AC/battery
status. Every reader degrades to `None` when a sysfs path is unavailable
(e.g. running in a container or on unsupported hardware) instead of
raising, since this is diagnostic data, not something the run should
fail over.
"""

from __future__ import annotations

import glob
import time
from pathlib import Path
from typing import Dict, List, Optional


def _read_int(path: str) -> Optional[int]:
    try:
        return int(Path(path).read_text().strip())
    except (OSError, ValueError):
        return None


def _read_str(path: str) -> Optional[str]:
    try:
        return Path(path).read_text().strip()
    except OSError:
        return None


def read_cpu_freq_mhz() -> Dict[str, Optional[float]]:
    """Mean/max current CPU frequency across all cores, in MHz."""
    freqs: List[int] = []
    for path in sorted(glob.glob("/sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq")):
        val = _read_int(path)
        if val is not None:
            freqs.append(val)

    if not freqs:
        return {"cpu_freq_mean_mhz": None, "cpu_freq_max_mhz": None}

    return {
        "cpu_freq_mean_mhz": (sum(freqs) / len(freqs)) / 1000.0,
        "cpu_freq_max_mhz": max(freqs) / 1000.0,
    }


def read_cpu_governor() -> Optional[str]:
    return _read_str("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")


def read_max_temp_celsius() -> Optional[float]:
    """Highest reading across all thermal zones, in degrees C."""
    readings: List[float] = []
    for path in sorted(glob.glob("/sys/class/thermal/thermal_zone*/temp")):
        val = _read_int(path)
        if val is not None:
            # Kernel reports millidegrees C on virtually all Linux drivers.
            readings.append(val / 1000.0)

    return max(readings) if readings else None


def read_memory_state() -> Dict[str, Optional[float]]:
    result: Dict[str, Optional[float]] = {
        "mem_total_kb": None,
        "mem_available_kb": None,
        "mem_free_kb": None,
        "swap_total_kb": None,
        "swap_free_kb": None,
    }
    try:
        lines = Path("/proc/meminfo").read_text().splitlines()
    except OSError:
        return result

    keymap = {
        "MemTotal:": "mem_total_kb",
        "MemAvailable:": "mem_available_kb",
        "MemFree:": "mem_free_kb",
        "SwapTotal:": "swap_total_kb",
        "SwapFree:": "swap_free_kb",
    }
    for line in lines:
        for prefix, key in keymap.items():
            if line.startswith(prefix):
                parts = line.split()
                if len(parts) >= 2:
                    try:
                        result[key] = float(parts[1])
                    except ValueError:
                        pass
    return result


def read_load_average() -> Dict[str, Optional[float]]:
    text = _read_str("/proc/loadavg")
    if not text:
        return {"load_1m": None, "load_5m": None, "load_15m": None}
    parts = text.split()
    if len(parts) < 3:
        return {"load_1m": None, "load_5m": None, "load_15m": None}
    try:
        return {
            "load_1m": float(parts[0]),
            "load_5m": float(parts[1]),
            "load_15m": float(parts[2]),
        }
    except ValueError:
        return {"load_1m": None, "load_5m": None, "load_15m": None}


def read_power_state() -> Dict[str, Optional[object]]:
    ac_online = _read_int("/sys/class/power_supply/AC/online")
    if ac_online is None:
        # Some kernels expose it as ADP1, ACAD, etc. Try a couple of common names.
        for name in ("ACAD", "ADP1", "AC0"):
            ac_online = _read_int(f"/sys/class/power_supply/{name}/online")
            if ac_online is not None:
                break

    battery_status = _read_str("/sys/class/power_supply/BAT0/status")
    battery_capacity = _read_int("/sys/class/power_supply/BAT0/capacity")

    return {
        "ac_online": None if ac_online is None else bool(ac_online),
        "battery_status": battery_status,
        "battery_capacity_pct": battery_capacity,
    }


def snapshot() -> Dict[str, object]:
    """Full machine-state snapshot, meant to be logged once per benchmark run."""
    state: Dict[str, object] = {"unix_time": time.time()}
    state.update(read_cpu_freq_mhz())
    state["cpu_governor"] = read_cpu_governor()
    state["temp_max_celsius"] = read_max_temp_celsius()
    state.update(read_memory_state())
    state.update(read_load_average())
    state.update(read_power_state())
    return state


STATE_FIELDNAMES = [
    "unix_time",
    "cpu_freq_mean_mhz",
    "cpu_freq_max_mhz",
    "cpu_governor",
    "temp_max_celsius",
    "mem_total_kb",
    "mem_available_kb",
    "mem_free_kb",
    "swap_total_kb",
    "swap_free_kb",
    "load_1m",
    "load_5m",
    "load_15m",
    "ac_online",
    "battery_status",
    "battery_capacity_pct",
]


if __name__ == "__main__":
    import json

    print(json.dumps(snapshot(), indent=2))
