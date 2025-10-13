use crate::models::ProcessStats;
use crate::services::ebpf_loader::EbpfLoader;
use sysinfo::{System, Pid, ProcessesToUpdate};
use std::sync::{Arc, Mutex};

pub struct StatsCollector {
    system: Mutex<System>,
    ebpf_loader: Arc<EbpfLoader>,  // ← 添加 eBPF loader 引用
}

impl StatsCollector {
    pub fn new(ebpf_loader: Arc<EbpfLoader>) -> Self {  // ← 接收 eBPF loader
        Self {
            system: Mutex::new(System::new_all()),
            ebpf_loader,
        }
    }

    pub async fn collect_stats(&self, pid: i32) -> Option<ProcessStats> {  // ← 改为 async
        let mut sys = self.system.lock().ok()?;

        let sysinfo_pid = Pid::from_u32(pid as u32);
        sys.refresh_processes(ProcessesToUpdate::All, true);

        let process = sys.process(sysinfo_pid)?;
        let total_memory = sys.total_memory();

        // *** 从 eBPF 读取网络统计 ***
        let network_stats = self.ebpf_loader.get_network_stats(pid).await;
        let (rx_bytes, tx_bytes, rx_packets, tx_packets) = if let Some(stats) = network_stats {
            (stats.rx_bytes, stats.tx_bytes, stats.rx_packets, stats.tx_packets)
        } else {
            (0, 0, 0, 0)
        };

        let stats = ProcessStats {
            cpu_usage: process.cpu_usage(),
            memory_bytes: process.memory(),
            memory_percent: if total_memory > 0 {
                (process.memory() as f32 / total_memory as f32) * 100.0
            } else {
                0.0
            },
            virtual_memory_bytes: process.virtual_memory(),
            disk_read_bytes: process.disk_usage().read_bytes,
            disk_written_bytes: process.disk_usage().written_bytes,
            thread_count: 0,
            network_rx_bytes: rx_bytes,
            network_tx_bytes: tx_bytes,
            network_rx_packets: rx_packets,
            network_tx_packets: tx_packets,
        };

        Some(stats)
    }
}
