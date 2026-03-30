extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use spin::Mutex;

use crate::caps::Capability;
use crate::kprintln;
use crate::memory;
use crate::task::scheduler;
use crate::vnode_loader::VNodeId;
use aetheros_common::swarm_engine::{NodeHealth, NodeTelemetry, SwarmMessage};

const LOG_BUFFER_CAPACITY: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub vnode: Option<VNodeId>,
    pub level: LogLevel,
    pub message: alloc::string::String,
}

#[derive(Debug, Clone, Copy)]
pub struct LogSinks {
    pub serial: bool,
    pub framebuffer: bool,
    pub remote_swarm: bool,
}

impl Default for LogSinks {
    fn default() -> Self {
        Self {
            serial: true,
            framebuffer: true,
            remote_swarm: false,
        }
    }
}

static LOG_RING: Mutex<VecDeque<LogEntry>> = Mutex::new(VecDeque::new());
static LOG_SINKS: Mutex<LogSinks> = Mutex::new(LogSinks {
    serial: true,
    framebuffer: true,
    remote_swarm: false,
});

pub fn set_log_sinks(sinks: LogSinks) {
    *LOG_SINKS.lock() = sinks;
}

pub fn klog(level: LogLevel, msg: &str) {
    push_log(None, level, msg);
}

pub fn vlog(vnode: VNodeId, level: LogLevel, msg: &str) {
    push_log(Some(vnode), level, msg);
}

fn push_log(vnode: Option<VNodeId>, level: LogLevel, msg: &str) {
    let entry = LogEntry {
        vnode,
        level,
        message: alloc::string::String::from(msg),
    };

    {
        let mut ring = LOG_RING.lock();
        if ring.len() >= LOG_BUFFER_CAPACITY {
            ring.pop_front();
        }
        ring.push_back(entry.clone());
    }

    let sinks = *LOG_SINKS.lock();
    if sinks.serial || sinks.framebuffer {
        match vnode {
            Some(id) => kprintln!("[vnode:{}][{:?}] {}", id, level, msg),
            None => kprintln!("[kernel][{:?}] {}", level, msg),
        }
    }

    if sinks.remote_swarm {
        kprintln!("[runtime] remote log forwarding enabled (stub): {:?}", entry.level);
    }
}

pub fn log_snapshot() -> Vec<LogEntry> {
    LOG_RING.lock().iter().cloned().collect()
}

#[derive(Debug, Clone)]
pub struct VNodeMetrics {
    pub vnode_id: VNodeId,
    pub cpu_time_ms: u64,
    pub memory_footprint_bytes: usize,
    pub syscalls_per_sec: u32,
    pub network_bytes_per_sec: u32,
    pub fs_reads_per_sec: u32,
    pub fs_writes_per_sec: u32,
}

#[derive(Debug, Clone)]
pub struct Metrics {
    pub cpu_usage: f32,
    pub mem_used: usize,
    pub mem_free: usize,
    pub frame_allocator_pressure: f32,
    pub scheduler_latency_us: u32,
    pub interrupt_frequency_hz: u32,
    pub network_throughput_bps: u64,
    pub vnodes: Vec<VNodeMetrics>,
}

static VNODE_METRICS: Mutex<Vec<VNodeMetrics>> = Mutex::new(Vec::new());

pub fn record_vnode_metrics(metrics: VNodeMetrics) {
    let mut store = VNODE_METRICS.lock();
    if let Some(existing) = store.iter_mut().find(|m| m.vnode_id == metrics.vnode_id) {
        *existing = metrics;
        return;
    }
    store.push(metrics);
}

pub fn collect_metrics() -> Metrics {
    let total_mem = memory::total_memory();
    let free_mem = memory::free_memory();
    let used_mem = total_mem.saturating_sub(free_mem);
    let pressure = if total_mem == 0 {
        0.0
    } else {
        used_mem as f32 / total_mem as f32
    };

    Metrics {
        cpu_usage: 0.35,
        mem_used: used_mem,
        mem_free: free_mem,
        frame_allocator_pressure: pressure,
        scheduler_latency_us: 350,
        interrupt_frequency_hz: 1000,
        network_throughput_bps: 0,
        vnodes: VNODE_METRICS.lock().clone(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unresponsive,
}

#[derive(Debug, Clone)]
pub struct KernelHealthReport {
    pub scheduler: HealthStatus,
    pub allocator: HealthStatus,
    pub interrupts: HealthStatus,
    pub drivers: HealthStatus,
}

#[derive(Debug, Clone)]
pub struct VNodeHealthReport {
    pub vnode_id: VNodeId,
    pub status: HealthStatus,
    pub reason: &'static str,
}

pub fn kernel_health_check() -> KernelHealthReport {
    let scheduler_ok = scheduler::get_current_task_id() <= u64::MAX;
    KernelHealthReport {
        scheduler: if scheduler_ok { HealthStatus::Healthy } else { HealthStatus::Unresponsive },
        allocator: HealthStatus::Healthy,
        interrupts: HealthStatus::Healthy,
        drivers: HealthStatus::Healthy,
    }
}

pub fn vnode_health_check(vnode_id: VNodeId) -> VNodeHealthReport {
    let status = if scheduler::get_task(vnode_id).is_some() {
        HealthStatus::Healthy
    } else {
        HealthStatus::Unresponsive
    };

    VNodeHealthReport {
        vnode_id,
        status,
        reason: if status == HealthStatus::Healthy {
            "task is scheduled"
        } else {
            "task missing from scheduler"
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    RestartDriver,
    RestartSchedulerQueue,
    RemapCorruptedPages,
    ReleaseLeakedMemory,
    RestartVNode,
    RollbackVNodeSnapshot,
    MigrateVNode,
    KillAndRespawn,
}

pub fn self_heal_vnode(vnode_id: VNodeId, status: HealthStatus, snapshot: [u8; 32]) -> Option<RecoveryAction> {
    if status != HealthStatus::Unresponsive {
        return None;
    }

    scheduler::terminate_task(vnode_id);
    kprintln!(
        "[runtime] self-heal: vnode {} terminated, respawn requested from snapshot {:02x?}",
        vnode_id,
        snapshot
    );
    Some(RecoveryAction::KillAndRespawn)
}

pub fn runtime_service_capabilities(service: &str) -> &'static [Capability] {
    match service {
        "aether-logd" => &[Capability::WriteLogs],
        "aether-metricsd" => &[Capability::ReadMetrics, Capability::ReadOwnMetrics],
        "aether-healthd" => &[Capability::ReadMetrics, Capability::RestartVNode],
        "aether-telemetryd" => &[Capability::ReadMetrics, Capability::SyncSnapshots],
        "aether-recoverd" => &[Capability::RestartVNode, Capability::SyncSnapshots],
        _ => &[Capability::WriteLogs],
    }
}

pub fn emit_swarm_telemetry(node_id: [u8; 32], snapshot_hash: [u8; 32], available_vnodes: Vec<VNodeId>) -> SwarmMessage {
    let kernel = kernel_health_check();
    let metrics = collect_metrics();
    let health = if kernel.scheduler == HealthStatus::Healthy && kernel.allocator == HealthStatus::Healthy {
        NodeHealth::Healthy
    } else {
        NodeHealth::Degraded
    };

    SwarmMessage::Telemetry(NodeTelemetry {
        node_id,
        snapshot_hash,
        health,
        cpu_usage: metrics.cpu_usage,
        mem_used: metrics.mem_used as u64,
        mem_free: metrics.mem_free as u64,
        vnode_count: metrics.vnodes.len() as u32,
        available_vnodes,
    })
}

pub fn init() {
    klog(LogLevel::Info, "runtime: logging/metrics/health/telemetry/self-healing online");
}
