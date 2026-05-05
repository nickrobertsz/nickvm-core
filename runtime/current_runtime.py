#!/usr/bin/env python3
import json, sys, traceback, subprocess, re, time
from datetime import datetime, timezone

SERVICES = {
    "theme": {
        "id": "core.theme",
        "version": "0.1.0",
        "component": "dripd"
    },

    "blackbox": {
        "id": "core.blackbox",
        "version": "0.1.0",
        "component": "blackboxd"
    },

    "registry": {
        "id": "core.registry",
        "version": "0.1.0",
        "component": "registryd"
    }
}

THEME = {
    "accent": "#4CC9F0",
    "mode": "dark"
}

def send(msg):
    sys.stdout.write(json.dumps(msg) + "\n")
    sys.stdout.flush()

def now():
    return datetime.now(timezone.utc).isoformat()

def hello():
    send({
        "timestamp": now(),
        "type": "hello",
        "component": "python_runtime",
        "status": "online"
    })

def handle(req):
    req_type = req.get("type")

    if req_type == "ping":
        return {
            "timestamp": now(),
            "type": "pong",
            "component": "python_runtime",
            "payload": "world"
        }

    if req_type == "fofoca_test":
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

            if latency < 0:
                verdict = "suspicious"
            elif latency < 50:
                verdict = "sexy"
            elif latency < 100:
                verdict = "acceptable"
            elif latency < 200:
                verdict = "brave"
            else:
                verdict = "naughty"

            return {
                "timestamp": now(),
                "type": "fofoca_event",
                "event": "network.ping_result",
                "host": "google.com",
                "latency_ms": latency,
                "verdict": verdict
            }

        except Exception as e:
            return {
                "timestamp": now(),
                "type": "error",
                "trace": str(e)
            }

    if req_type == "fofoca_stream":
        for i in range(15):
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

                if latency < 0:
                    verdict = "suspicious"
                elif latency < 50:
                    verdict = "sexy"
                elif latency < 100:
                    verdict = "acceptable"
                elif latency < 200:
                    verdict = "brave"
                else:
                    verdict = "naughty"

                send({
                    "timestamp": now(),
                    "type": "fofoca_event",
                    "event": "network.ping_result",
                    "host": "google.com",
                    "latency_ms": latency,
                    "sequence": i + 1,
                    "verdict": verdict
                })

            except Exception as e:
                send({
                    "timestamp": now(),
                    "type": "error",
                    "trace": str(e)
                })

            time.sleep(2)

        return {
            "timestamp": now(),
            "type": "fofoca_stream_complete"
        }

    if req_type == "whois":
        name = req.get("name")
        return {
            "timestamp": now(),
            "type": "whois_res",
            "name": name,
            "data": SERVICES.get(name)
        }
    if req_type == "services":
        return {
            "timestamp": now(),
            "type": "services_res",
            "services": list(SERVICES.keys())
        }

    if req_type == "theme_get":
        return {
            "timestamp": now(),
            "type": "theme_state",
            "theme": THEME
        }

    if req_type == "theme_set":
        accent = req.get("accent")
        if accent:
            THEME["accent"] = accent
        return {
            "timestamp": now(),
            "type": "theme_state",
            "theme": THEME
        }

    if req_type == "update_check":
        return {
            "timestamp": now(),
            "type": "update_status",
            "available": False,
            "source": "none"
        }

    if req_type == "crash_me":
        raise Exception("intentional crash for testing")

def main():
    hello()

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
            res = handle(req)
            send(res)
        except Exception:
            send({
                "timestamp": now(),
                "type": "error",
                "trace": traceback.format_exc()
            })

if __name__ == "__main__":
    main()
