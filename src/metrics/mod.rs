use prometheus::{
    Encoder, GaugeVec, CounterVec, Opts, Registry, TextEncoder,
    register_gauge_vec_with_registry, register_counter_vec_with_registry,
};
use lazy_static::lazy_static;
use std::sync::Arc;
use sysinfo::System;

pub struct MetricsRegistry {
    registry: Registry,

    // Gauge metrics
    pub process_up: GaugeVec,
    pub process_pid_info: GaugeVec,
    pub process_cpu_usage: GaugeVec,
    pub process_memory_bytes: GaugeVec,
    pub process_memory_percent: GaugeVec,
    pub process_virtual_memory_bytes: GaugeVec,
    pub process_thread_count: GaugeVec,
    pub process_registered_timestamp: GaugeVec,
    pub process_last_check_timestamp: GaugeVec,

    // Counter metrics
    pub process_disk_read_bytes: CounterVec,
    pub process_disk_written_bytes: CounterVec,
    pub process_network_tx_bytes: CounterVec,
    pub process_network_rx_bytes: CounterVec,
    pub process_network_tx_packets: CounterVec,
    pub process_network_rx_packets: CounterVec,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        let registry = Registry::new();

        // 定义通用的标签
        let common_labels = &["name", "cmdline","hostname"];

        // Gauge metrics
        let process_up = register_gauge_vec_with_registry!(
            Opts::new("process_up", "Process is running (1) or down (0)"),
            common_labels,
            registry
        ).unwrap();

        let process_pid_info = register_gauge_vec_with_registry!(
            Opts::new("process_pid_info", "Process PID information"),
            &["name", "pid", "hostname"],
            registry
        ).unwrap();

        let process_cpu_usage = register_gauge_vec_with_registry!(
            Opts::new("process_cpu_usage_percent", "Process CPU usage percentage"),
            common_labels,
            registry
        ).unwrap();

        let process_memory_bytes = register_gauge_vec_with_registry!(
            Opts::new("process_memory_bytes", "Process memory usage in bytes"),
            common_labels,
            registry
        ).unwrap();

        let process_memory_percent = register_gauge_vec_with_registry!(
            Opts::new("process_memory_percent", "Process memory usage percentage"),
            common_labels,
            registry
        ).unwrap();

        let process_virtual_memory_bytes = register_gauge_vec_with_registry!(
            Opts::new("process_virtual_memory_bytes", "Process virtual memory in bytes"),
            common_labels,
            registry
        ).unwrap();

        let process_thread_count = register_gauge_vec_with_registry!(
            Opts::new("process_thread_count", "Number of threads"),
            common_labels,
            registry
        ).unwrap();

        let process_registered_timestamp = register_gauge_vec_with_registry!(
            Opts::new("process_registered_timestamp_seconds", "Unix timestamp when process was registered"),
            common_labels,
            registry
        ).unwrap();

        let process_last_check_timestamp = register_gauge_vec_with_registry!(
            Opts::new("process_last_check_timestamp_seconds", "Unix timestamp of last process check"),
            common_labels,
            registry
        ).unwrap();

        // Counter metrics
        let process_disk_read_bytes = register_counter_vec_with_registry!(
            Opts::new("process_disk_read_bytes", "Total disk read bytes"),
            common_labels,
            registry
        ).unwrap();

        let process_disk_written_bytes = register_counter_vec_with_registry!(
            Opts::new("process_disk_written_bytes", "Total disk written bytes"),
            common_labels,
            registry
        ).unwrap();

        let process_network_tx_bytes = register_counter_vec_with_registry!(
            Opts::new("process_network_tx_bytes", "Network transmitted bytes"),
            common_labels,
            registry
        ).unwrap();

        let process_network_rx_bytes = register_counter_vec_with_registry!(
            Opts::new("process_network_rx_bytes", "Network received bytes"),
            common_labels,
            registry
        ).unwrap();

        let process_network_tx_packets = register_counter_vec_with_registry!(
            Opts::new("process_network_tx_packets", "Network transmitted packets"),
            common_labels,
            registry
        ).unwrap();

        let process_network_rx_packets = register_counter_vec_with_registry!(
            Opts::new("process_network_rx_packets", "Network received packets"),
            common_labels,
            registry
        ).unwrap();

        Self {
            registry,
            process_up,
            process_pid_info,
            process_cpu_usage,
            process_memory_bytes,
            process_memory_percent,
            process_virtual_memory_bytes,
            process_thread_count,
            process_registered_timestamp,
            process_last_check_timestamp,
            process_disk_read_bytes,
            process_disk_written_bytes,
            process_network_tx_bytes,
            process_network_rx_bytes,
            process_network_tx_packets,
            process_network_rx_packets,
        }
    }

    pub fn render(&self) -> Result<String, Box<dyn std::error::Error>> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }

    pub fn reset_process_metrics(&self, name: &str, cmdline: &str) {
        // 重置所有该进程的 metrics
        let hostname = System::host_name().unwrap_or_else(|| "unknown".to_string());
        let labels = &[name, cmdline, &hostname.clone()];

        // 删除旧的 metric 值
        let _ = self.process_pid_info.remove_label_values(&[name, "0", &hostname.clone()]);
        let _ = self.process_up.remove_label_values(labels);
        let _ = self.process_cpu_usage.remove_label_values(labels);
        let _ = self.process_memory_bytes.remove_label_values(labels);
        let _ = self.process_memory_percent.remove_label_values(labels);
        let _ = self.process_virtual_memory_bytes.remove_label_values(labels);
        let _ = self.process_thread_count.remove_label_values(labels);
        let _ = self.process_registered_timestamp.remove_label_values(labels);
        let _ = self.process_last_check_timestamp.remove_label_values(labels);
    }
}

lazy_static! {
    pub static ref METRICS: Arc<MetricsRegistry> = Arc::new(MetricsRegistry::new());
}