# NPU Data Exporter (`npu-data-exporter`)

`npu-data-exporter` is a lightweight, standalone system telemetry daemon written in Rust. It captures real-time hardware metrics and per-process execution statistics for AMD XDNA NPUs (Neural Processing Units) and exposes them via atomic, world-readable files in `/run/` and `/tmp/`.

---

## Key Use Cases

### 1. Sandboxed Desktop Applications (Flatpak / Snap)
Sandboxed graphical applications such as GNOME Resources are restricted by Linux container security policies from accessing low-level hardware debug interfaces (`/sys/kernel/debug/accel/`) or reading `/proc/<PID>/fdinfo` across namespace boundaries.
- `npu-data-exporter` runs on the host with root privileges and outputs atomic telemetry files (`0644` permissions) that sandboxed applications can safely read without compromising sandbox isolation.

### 2. Multi-User & System Service Process Monitoring
Standard user applications can only inspect process file descriptors (`/proc/<PID>/fdinfo`) belonging to their own user account. AI workloads running under root or dedicated service accounts (such as `ollama`, `vllm`, `llama.cpp`, or background system daemons) are hidden from unprivileged user queries.
- `npu-data-exporter` aggregates NPU execution nanoseconds and memory allocations across **all** system processes regardless of user ownership, ensuring complete system-wide process visibility.

### 3. External System Monitoring & Dashboards
The exporter generates standardized JSON (`/run/amdxdna_npu.json`) and raw gauge files that can be consumed directly by monitoring agents (such as Prometheus node-exporter, Grafana, Cockpit, or custom status bar scripts) independently of GNOME Resources.

---

## Exported Telemetry Files

`npu-data-exporter` writes atomic, world-readable (`0644`) files updated every 1000ms:

| File Path | Description | Example Content |
|---|---|---|
| `/run/npu_busy_percent` | Overall NPU hardware busy percentage | `14.8` |
| `/run/npu_memory_bytes` | Total allocated NPU memory in bytes | `8892112896` |
| `/run/npu_temperature_celsius` | NPU / APU package temperature in °C | `68.6` |
| `/run/npu_power_watts` | Real-time NPU power draw in Watts | `45.0` |
| `/run/npu_power_state` | Hardware runtime power status | `active` |
| `/run/amdxdna_npu.json` | Detailed system & per-process telemetry JSON | See schema below |

### JSON Telemetry Schema (`/run/amdxdna_npu.json`)
```json
{
  "busy_percent": 14.87,
  "used_memory_bytes": 8892112896,
  "temperature_celsius": 68.6,
  "power_watts": 45.0,
  "power_state": "active",
  "timestamp": 1788763924,
  "processes": [
    {
      "pid": 3386754,
      "comm": "flm-real",
      "user": "unknown",
      "busy_percent": 14.87,
      "memory_bytes": 8873467904,
      "proc_ns": 2325200000000
    }
  ]
}
```

---

## Command-Line Usage

### Manual Execution
Run directly from terminal (requires `sudo` for full system `/proc` and `debugfs` access):

```bash
sudo ./build/src/release/npu-data-exporter [OPTIONS]
```

### Options
```text
Usage: npu-data-exporter [OPTIONS]

Options:
  -i, --interval <INTERVAL>  Sampling refresh interval in milliseconds [default: 1000]
  -h, --help                 Print help information
```

---

## Systemd Service Setup

To run `npu-data-exporter` as a persistent background daemon:

1. Copy the compiled binary to `/usr/local/bin/`:
   ```bash
   sudo cp ./build/src/release/npu-data-exporter /usr/local/bin/
   ```

2. Create `/etc/systemd/system/npu-data-exporter.service`:
   ```ini
   [Unit]
   Description=AMD XDNA NPU Telemetry Exporter Daemon
   After=multi-user.target

   [Service]
   Type=simple
   ExecStart=/usr/local/bin/npu-data-exporter --interval 1000
   Restart=always
   RestartSec=3

   [Install]
   WantedBy=multi-user.target
   ```

3. Enable and start the service:
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable --now npu-data-exporter
   ```
