import json
import subprocess
import re
import time
from datetime import datetime, timezone

def now():
    return datetime.now(timezone.utc).isoformat()

def verdict_for_latency(latency):
    if latency < 0:
        return "suspicious"
    if latency < 50:
        return "sexy"
    if latency < 100:
        return "acceptable"
    if latency < 200:
        return "brave"
    return "naughty"

print("PingGoblin.tia awakening...\n", flush=True)

for i in range(5):
    try:
        result = subprocess.run(
            ["ping", "-c", "1", "google.com"],
            capture_output=True,
            text=True,
            timeout=5
        )

        output = result.stdout

        match = re.search(r'time=(\d+\.\d+)', output)

        latency = float(match.group(1)) if match else -1

        event = {
            "timestamp": now(),
            "type": "fofoca_event",
            "source": "PingGoblin.tia",
            "event": "network.ping_result",
            "host": "google.com",
            "latency_ms": latency,
            "sequence": i + 1,
            "verdict": verdict_for_latency(latency)
        }

        print(json.dumps(event), flush=True)

    except Exception as e:
        print(json.dumps({
            "timestamp": now(),
            "type": "error",
            "source": "PingGoblin.tia",
            "trace": str(e)
        }), flush=True)

    time.sleep(2)

print("\nPingGoblin.tia entering goblin sleep.", flush=True)
