use actix_web::{web, HttpResponse, Responder};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::services::{check_process_running, get_process_pid};
use crate::state::AppState;
use crate::metrics::METRICS;

pub async fn get_metrics(data: web::Data<AppState>) -> impl Responder {
    let mut state = data.lock().unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // 克隆 ebpf_loader 避免借用冲突
    let ebpf_loader = state.ebpf_loader.clone();

    // 先收集需要更新的进程信息
    let pids_to_update: Vec<(String, Option<i32>, String)> = state.processes.iter()
        .map(|(name, status)| (name.clone(), status.pid, status.config.cmdline.clone()))
        .collect();

    // 更新每个进程的状态和统计
    for (name, _old_pid, cmdline) in pids_to_update {
        // 检查进程状态
        let is_running = check_process_running(&cmdline);
        let pid = get_process_pid(&cmdline);

        // 收集基础统计（CPU、内存等）- 异步操作
        let stats = if let Some(p) = pid {
            let ebpf_loader_clone = ebpf_loader.clone();
            drop(state);  // 释放锁以便执行异步操作
            
            // 创建临时的 stats_collector
            let temp_collector = crate::services::StatsCollector::new(ebpf_loader_clone);
            let collected = temp_collector.collect_stats(p).await;
            
            state = data.lock().unwrap();  // 重新获取锁
            collected
        } else {
            None
        };

        // 更新状态
        if let Some(status) = state.processes.get_mut(&name) {
            status.is_running = is_running;
            status.pid = pid;
            status.last_check = now;

            // 更新基础统计
            if let Some(s) = stats {
                status.stats = s;
            }
        }
    }

    // 使用 Prometheus SDK 更新 metrics
    for (_, status) in state.processes.iter() {
        let name = &status.config.name;
        let cmdline = &status.config.cmdline;
        let labels = &[name.as_str(), cmdline.as_str()];

        // PID info
        if let Some(pid) = status.pid {
            METRICS.process_pid_info
                .with_label_values(&[name.as_str(), &pid.to_string()])
                .set(1.0);
        }

        // process_up
        METRICS.process_up
            .with_label_values(labels)
            .set(if status.is_running { 1.0 } else { 0.0 });

        // 只有进程运行时才输出资源 metrics
        if status.is_running && status.stats.is_valid() {
            // CPU
            METRICS.process_cpu_usage
                .with_label_values(labels)
                .set(status.stats.cpu_usage as f64);

            // Memory
            METRICS.process_memory_bytes
                .with_label_values(labels)
                .set(status.stats.memory_bytes as f64);

            METRICS.process_memory_percent
                .with_label_values(labels)
                .set(status.stats.memory_percent as f64);

            METRICS.process_virtual_memory_bytes
                .with_label_values(labels)
                .set(status.stats.virtual_memory_bytes as f64);

            // Thread count
            METRICS.process_thread_count
                .with_label_values(labels)
                .set(status.stats.thread_count as f64);

            // Disk I/O - 注意：Counter 需要特殊处理
            // 我们需要重置并设置为当前值
            let _ = METRICS.process_disk_read_bytes.remove_label_values(labels);
            METRICS.process_disk_read_bytes
                .with_label_values(labels)
                .inc_by(status.stats.disk_read_bytes as f64);

            let _ = METRICS.process_disk_written_bytes.remove_label_values(labels);
            METRICS.process_disk_written_bytes
                .with_label_values(labels)
                .inc_by(status.stats.disk_written_bytes as f64);

            // Network - eBPF 统计
            let _ = METRICS.process_network_tx_bytes.remove_label_values(labels);
            METRICS.process_network_tx_bytes
                .with_label_values(labels)
                .inc_by(status.stats.network_tx_bytes as f64);

            let _ = METRICS.process_network_rx_bytes.remove_label_values(labels);
            METRICS.process_network_rx_bytes
                .with_label_values(labels)
                .inc_by(status.stats.network_rx_bytes as f64);

            let _ = METRICS.process_network_tx_packets.remove_label_values(labels);
            METRICS.process_network_tx_packets
                .with_label_values(labels)
                .inc_by(status.stats.network_tx_packets as f64);

            let _ = METRICS.process_network_rx_packets.remove_label_values(labels);
            METRICS.process_network_rx_packets
                .with_label_values(labels)
                .inc_by(status.stats.network_rx_packets as f64);
        }

        // Timestamps
        METRICS.process_registered_timestamp
            .with_label_values(labels)
            .set(status.registered_at as f64);

        METRICS.process_last_check_timestamp
            .with_label_values(labels)
            .set(status.last_check as f64);
    }

    // 释放锁
    drop(state);

    // 渲染 Prometheus metrics
    match METRICS.render() {
        Ok(metrics_text) => HttpResponse::Ok()
            .content_type("text/plain; version=0.0.4")
            .body(metrics_text),
        Err(e) => {
            log::error!("Failed to render metrics: {}", e);
            HttpResponse::InternalServerError().body("Failed to render metrics")
        }
    }
}
