import json
import sys

def judge(latency, verdict):
    if latency < 0:
        return "hmmm timeout... suspicious goblin behavior 💀"
    if verdict == "sexy":
        return f"hmmm {latency}ms... sexy 😏"
    if verdict == "acceptable":
        return f"hmmm {latency}ms... acceptable. Respectable auntie-grade packet."
    if verdict == "brave":
        return f"hmmm {latency}ms... brave. Little packet fought for its life."
    if verdict == "naughty":
        return f"hmmm {latency}ms... naughty. Google is walking through mud."
    return f"hmmm {latency}ms... unknown vibes."

print("PingJudge.tia listening for Fofoca...\n", flush=True)

for line in sys.stdin:
    line = line.strip()

    if not line:
        continue

    try:
        event = json.loads(line)

        if event.get("type") != "fofoca_event":
            continue

        if event.get("event") != "network.ping_result":
            continue

        latency = event.get("latency_ms", -1)
        verdict = event.get("verdict", "unknown")
        sequence = event.get("sequence", "?")
        host = event.get("host", "unknown")

        print(f"[{sequence}] {host}: {judge(latency, verdict)}", flush=True)

    except json.JSONDecodeError:
        continue
