//! # NPU Data Exporter (`npu-data-exporter`)
//!
//! A standalone telemetry daemon for AMD XDNA Neural Processing Units (NPUs).
//!
//! This service periodically collects low-level hardware metrics (temperature, power,
//! active memory, runtime power state) and per-process NPU execution nanoseconds across
//! `/proc/*/fdinfo` and sysfs hwmon nodes.
//!
//! It writes atomic, world-readable (`0644`) files to `/run/` (with fallback to `/tmp/`)
//! so sandboxed graphical applications (such as Flatpak GNOME Resources) and external monitoring
//! daemons can consume telemetry without needing direct hardware device node access.

use anyhow::Result;
use clap::Parser;
use log::{info, trace};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Write,
    os::unix::fs::PermissionsExt,
    path::Path,
    thread,
    time::{Duration, Instant},
};

/// Command-line arguments for `npu-data-exporter`.
#[derive(Parser, Debug)]
#[command(version, about = "AMD XDNA NPU Hardware & Process Telemetry Exporter")]
struct Args {
    /// Telemetry sampling refresh interval in milliseconds.
    #[arg(short, long, default_value_t = 1000)]
    interval: u64,

    /// Output telemetry JSON once to stdout and exit immediately.
    #[arg(short, long, default_value_t = false)]
    once: bool,
}

/// Telemetry details for an active process consuming NPU hardware resources.
#[derive(Debug, Serialize, Deserialize, Default)]
struct ProcessNpuInfo {
    /// Process ID (PID).
    pid: libc::pid_t,
    /// Process executable comm string.
    comm: String,
    /// Username running the process.
    user: String,
    /// Calculated busy percentage for this process.
    busy_percent: f64,
    /// NPU memory allocated in bytes.
    memory_bytes: u64,
    /// Total cumulative execution time on NPU in nanoseconds.
    proc_ns: u64,
}

/// System-wide telemetry payload serialized to `/run/amdxdna_npu.json`.
#[derive(Debug, Serialize, Default)]
struct AmdxdnaTelemetry {
    /// Overall NPU hardware busy percentage (0.0 to 100.0%).
    busy_percent: f64,
    /// Total active NPU memory allocated in bytes across all processes.
    used_memory_bytes: u64,
    /// Temperature reading in degrees Celsius.
    temperature_celsius: Option<f64>,
    /// Power usage reading in Watts.
    power_watts: Option<f64>,
    /// Hardware runtime power state string (e.g. "active", "suspended").
    power_state: String,
    /// UNIX timestamp of telemetry collection in seconds.
    timestamp: u64,
    /// List of processes active on the NPU.
    processes: Vec<ProcessNpuInfo>,
}

/// Atomically writes content to `path` with `0644` permissions using a temporary file.
///
/// If writing to the primary path (e.g., in `/run`) fails due to permissions, it automatically
/// falls back to writing to `/tmp/`.
fn atomic_write<P: AsRef<Path>>(path: P, content: &str) -> Result<()> {
    let primary_path = path.as_ref();
    let parent = primary_path.parent().unwrap_or_else(|| Path::new("/run"));
    let tmp_file_name = format!(".{}.tmp", primary_path.file_name().unwrap().to_string_lossy());
    let primary_tmp = parent.join(&tmp_file_name);

    if let Ok(mut file) = File::create(&primary_tmp) {
        let _ = file.write_all(content.as_bytes());
        let _ = file.flush();
        if let Ok(meta) = file.metadata() {
            let mut perms = meta.permissions();
            perms.set_mode(0o644);
            let _ = file.set_permissions(perms);
        }
        if fs::rename(&primary_tmp, primary_path).is_ok() {
            return Ok(());
        }
    }

    // Fallback to /tmp if primary directory is unwritable
    let fallback_path = Path::new("/tmp").join(primary_path.file_name().unwrap());
    let fallback_tmp = Path::new("/tmp").join(&tmp_file_name);
    let mut file = File::create(&fallback_tmp)?;
    file.write_all(content.as_bytes())?;
    file.flush()?;
    let mut perms = file.metadata()?.permissions();
    perms.set_mode(0o644);
    file.set_permissions(perms)?;
    fs::rename(fallback_tmp, fallback_path)?;

    Ok(())
}

/// Scans `/proc/*/fdinfo/` entries for active `amdxdna` DRM file descriptors.
///
/// Returns total execution nanoseconds, total allocated memory, and per-process NPU metrics.
fn scan_fdinfos() -> (u64, u64, Vec<ProcessNpuInfo>) {
    let mut total_ns = 0u64;
    let mut total_mem = 0u64;
    let mut proc_list = Vec::new();

    let Ok(proc_dir) = fs::read_dir("/proc") else {
        return (0, 0, proc_list);
    };

    for entry in proc_dir.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        let Ok(pid) = name_str.parse::<libc::pid_t>() else {
            continue;
        };

        let fdinfo_dir = entry.path().join("fdinfo");
        let Ok(fdinfo_entries) = fs::read_dir(fdinfo_dir) else {
            continue;
        };

        let mut proc_ns = 0u64;
        let mut proc_mem = 0u64;
        let mut is_npu = false;

        for fd_entry in fdinfo_entries.flatten() {
            let Ok(content) = fs::read_to_string(fd_entry.path()) else {
                continue;
            };

            let mut is_amdxdna = false;
            let mut ns_val = 0u64;
            let mut mem_val = 0u64;

            for line in content.lines() {
                if let Some((k, v)) = line.split_once(':') {
                    let key = k.trim();
                    let val = v.trim();

                    if key == "drm-driver" && (val == "amdxdna" || val == "amdxdna_accel_driver") {
                        is_amdxdna = true;
                    }

                    if key == "drm-engine-npu-amdxdna"
                        || key == "drm-engine-npu"
                        || key == "drm-engine-accel"
                    {
                        if let Some(ns_str) = val.split_whitespace().next() {
                            ns_val = ns_str.parse().unwrap_or(0);
                        }
                    }

                    if key == "drm-total-memory" || key == "drm-memory-vram" {
                        if let Some(kb_str) = val.split_whitespace().next() {
                            let kb: u64 = kb_str.parse().unwrap_or(0);
                            mem_val = kb * 1024;
                        }
                    }
                }
            }

            if is_amdxdna {
                is_npu = true;
                proc_ns = proc_ns.max(ns_val);
                proc_mem = proc_mem.max(mem_val);
            }
        }

        if is_npu {
            let comm = fs::read_to_string(entry.path().join("comm"))
                .unwrap_or_default()
                .trim()
                .to_string();

            // Exclude telemetry exporters and resources GUI from NPU execution calculation
            if comm.contains("exporter") || comm.contains("resources") {
                continue;
            }

            total_ns += proc_ns;
            total_mem += proc_mem;

            proc_list.push(ProcessNpuInfo {
                pid,
                comm,
                user: "unknown".into(),
                busy_percent: 0.0,
                memory_bytes: proc_mem,
                proc_ns,
            });
        }
    }

    (total_ns, total_mem, proc_list)
}

/// Fallback process scanner when fdinfo permissions are restricted.
///
/// Scans known NPU workload executable comm/cmdline patterns.
fn scan_process_fallback() -> (u64, u64, Vec<ProcessNpuInfo>) {
    let mut total_ns = 0u64;
    let mut total_mem = 0u64;
    let mut proc_list = Vec::new();

    let Ok(proc_dir) = fs::read_dir("/proc") else {
        return (0, 0, proc_list);
    };

    for entry in proc_dir.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        let Ok(pid) = name_str.parse::<libc::pid_t>() else {
            continue;
        };

        let comm = fs::read_to_string(entry.path().join("comm"))
            .unwrap_or_default()
            .trim()
            .to_string();
        let cmdline = fs::read_to_string(entry.path().join("cmdline"))
            .unwrap_or_default()
            .replace('\0', " ");

        let is_exporter = comm.contains("exporter")
            || cmdline.contains("exporter")
            || comm.contains("resources")
            || cmdline.contains("resources");

        let is_npu_worker = !is_exporter
            && (comm == "flm-real"
                || cmdline.contains("flm-real")
                || cmdline.contains("/npu/")
                || comm.contains("xrt")
                || cmdline.contains("xrt_")
                || cmdline.contains("xrt-")
                || cmdline.contains("lemonade_test"));

        if is_npu_worker {
            let stat_str = fs::read_to_string(entry.path().join("stat")).unwrap_or_default();
            let parts: Vec<&str> = stat_str.split_whitespace().collect();
            let mut proc_ns = 0u64;
            if parts.len() > 14 {
                let utime: u64 = parts[13].parse().unwrap_or(0);
                let stime: u64 = parts[14].parse().unwrap_or(0);
                proc_ns = (utime + stime).saturating_mul(10_000_000);
            }

            let mut proc_mem = 0u64;
            if let Ok(status_str) = fs::read_to_string(entry.path().join("status")) {
                for line in status_str.lines() {
                    if line.starts_with("VmRSS:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            let kb: u64 = kb_str.parse().unwrap_or(0);
                            proc_mem = kb * 1024;
                        }
                    }
                }
            }

            total_ns += proc_ns;
            total_mem += proc_mem;

            proc_list.push(ProcessNpuInfo {
                pid,
                comm,
                user: "unknown".into(),
                busy_percent: 0.0,
                memory_bytes: proc_mem,
                proc_ns,
            });
        }
    }

    (total_ns, total_mem, proc_list)
}

/// Reads thermal temperature inputs from sysfs hwmon nodes (`amdxdna`, `k10temp`, `amdgpu`, `acpitz`).
fn read_hwmon_temp() -> Option<f64> {
    let Ok(entries) = fs::read_dir("/sys/class/hwmon") else {
        return None;
    };
    let mut fallback_temp = None;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = fs::read_to_string(path.join("name"))
            .unwrap_or_default()
            .trim()
            .to_string();
        let is_priority = name == "amdxdna" || name == "k10temp";
        let is_candidate = is_priority || name == "amdgpu" || name == "acpitz";
        if is_candidate {
            if let Ok(hw_entries) = fs::read_dir(&path) {
                for file_entry in hw_entries.flatten() {
                    let fname = file_entry.file_name();
                    let fname_str = fname.to_string_lossy();
                    if fname_str.starts_with("temp") && fname_str.ends_with("_input") {
                        if let Ok(val_str) = fs::read_to_string(file_entry.path()) {
                            if let Ok(mdeg) = val_str.trim().parse::<f64>() {
                                let deg = mdeg / 1000.0;
                                if is_priority {
                                    return Some(deg);
                                } else if fallback_temp.is_none() {
                                    fallback_temp = Some(deg);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    fallback_temp
}

/// Reads power draw inputs in Watts from sysfs hwmon nodes (`power1_average`, `power1_input`).
fn read_hwmon_power() -> Option<f64> {
    let Ok(entries) = fs::read_dir("/sys/class/hwmon") else {
        return None;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        for fname in &["power1_average", "power1_input"] {
            let pfile = path.join(fname);
            if pfile.exists() {
                if let Ok(val_str) = fs::read_to_string(&pfile) {
                    if let Ok(uw) = val_str.trim().parse::<f64>() {
                        if uw > 0.0 {
                            return Some(uw / 1_000_000.0);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Reads runtime power state of the NPU device from `/sys/class/accel/accel0/power/runtime_status`.
fn read_power_state() -> String {
    fs::read_to_string("/sys/class/accel/accel0/power/runtime_status")
        .unwrap_or_else(|_| "active".into())
        .trim()
        .to_string()
}

fn main() -> Result<()> {
    pretty_env_logger::init();
    info!("Starting NPU Data Exporter Daemon…");

    let args = Args::parse();
    let interval = Duration::from_millis(args.interval);

    let output_percent_path = "/run/npu_busy_percent";
    let output_memory_path = "/run/npu_memory_bytes";
    let output_temp_path = "/run/npu_temperature_celsius";
    let output_power_path = "/run/npu_power_watts";
    let output_state_path = "/run/npu_power_state";
    let output_json_path = "/run/amdxdna_npu.json";

    let mut last_ns = 0u64;
    let mut last_time = Instant::now();

    loop {
        let now = Instant::now();
        let elapsed_secs = now.duration_since(last_time).as_secs_f64();
        last_time = now;

        let (fd_ns, fd_mem, mut proc_list) = scan_fdinfos();
        let (proc_ns, proc_mem, fallback_procs) = if fd_ns == 0 {
            scan_process_fallback()
        } else {
            (0, 0, Vec::new())
        };

        let current_ns = if fd_ns > 0 { fd_ns } else { proc_ns };
        let current_mem = if fd_mem > 0 { fd_mem } else { proc_mem };
        if proc_list.is_empty() {
            proc_list = fallback_procs;
        }

        let busy_percent = if last_ns > 0 && current_ns >= last_ns && elapsed_secs > 0.0 {
            let delta_ns = current_ns - last_ns;
            let delta_secs = delta_ns as f64 / 1_000_000_000.0;
            ((delta_secs / elapsed_secs) * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };
        last_ns = current_ns;

        for p in &mut proc_list {
            p.busy_percent = busy_percent;
        }

        let power_state = read_power_state();
        let temp_celsius = read_hwmon_temp();
        let power_watts = read_hwmon_power();

        let unix_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let telemetry = AmdxdnaTelemetry {
            busy_percent,
            used_memory_bytes: current_mem,
            temperature_celsius: temp_celsius,
            power_watts,
            power_state: power_state.clone(),
            timestamp: unix_timestamp,
            processes: proc_list,
        };

        let json_str = serde_json::to_string_pretty(&telemetry)?;

        let _ = atomic_write(output_percent_path, &format!("{:.1}\n", busy_percent));
        let _ = atomic_write(output_memory_path, &format!("{}\n", current_mem));
        if let Some(temp) = temp_celsius {
            let _ = atomic_write(output_temp_path, &format!("{:.1}\n", temp));
        }
        if let Some(power) = power_watts {
            let _ = atomic_write(output_power_path, &format!("{:.2}\n", power));
        }
        let _ = atomic_write(output_state_path, &format!("{}\n", power_state));
        let _ = atomic_write(output_json_path, &json_str);

        trace!(
            "Exported telemetry: busy={:.1}%, mem={}B, temp={:?}C, power={:?}W, state={}",
            busy_percent, current_mem, temp_celsius, power_watts, power_state
        );

        if args.once {
            println!("{json_str}");
            return Ok(());
        }

        thread::sleep(interval);
    }
}
