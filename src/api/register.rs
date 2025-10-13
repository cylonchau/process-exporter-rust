use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{ProcessConfig, ProcessStatus, ProcessStats};
use crate::services::{check_process_running, get_process_pid, get_all_matching_pids};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub cmdline: String,
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

pub async fn register_process(
    data: web::Data<AppState>,
    req: web::Json<RegisterRequest>,
) -> impl Responder {
    let mut state = data.lock().unwrap();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let is_running = check_process_running(&req.cmdline);
    let pid = get_process_pid(&req.cmdline);

    // 添加详细调试日志
    log::info!("=== Process Registration Debug ===");
    log::info!("  Requested cmdline: '{}'", req.cmdline);
    log::info!("  Is running: {}", is_running);
    log::info!("  Found PID: {:?}", pid);

    // 列出所有匹配的 PIDs
    let all_pids = get_all_matching_pids(&req.cmdline);
    log::info!("  All matching PIDs: {:?}", all_pids);

    // 检查 PID 是否已被注册
    if let Some(current_pid) = pid {
        for (existing_name, existing_status) in state.processes.iter() {
            if let Some(existing_pid) = existing_status.pid {
                if existing_pid == current_pid && existing_name != &req.name {
                    return HttpResponse::Conflict().json(serde_json::json!({
                        "status": "error",
                        "message": format!("Process with PID {} is already registered as '{}'", current_pid, existing_name),
                        "existing_name": existing_name,
                        "pid": current_pid
                    }));
                }
            }
        }
    }

    // 收集进程统计信息
    let stats = if let Some(p) = pid {
        // 克隆 ebpf_loader，避免借用冲突
        let ebpf_loader = state.ebpf_loader.clone();
        drop(state);  // 释放锁以便执行异步操作
        
        // 创建临时的 stats_collector
        let temp_collector = crate::services::StatsCollector::new(ebpf_loader);
        let collected_stats = temp_collector.collect_stats(p).await.unwrap_or_default();
        
        state = data.lock().unwrap();  // 重新获取锁
        collected_stats
    } else {
        ProcessStats::empty()
    };

    let config = ProcessConfig {
        name: req.name.clone(),
        cmdline: req.cmdline.clone(),
        labels: req.labels.clone(),
    };

    let status = ProcessStatus {
        config: config.clone(),
        registered_at: now,
        last_check: now,
        is_running,
        pid,
        stats: stats.clone(),
    };

    let final_status = if let Some(existing) = state.processes.get(&req.name) {
        ProcessStatus {
            registered_at: existing.registered_at,
            ..status
        }
    } else {
        status
    };

    state.processes.insert(req.name.clone(), final_status);

    // *** 添加到 eBPF 白名单 ***
    if let Some(p) = pid {
        log::info!("  Adding PID {} to eBPF whitelist", p);
        let ebpf_loader = state.ebpf_loader.clone();
        drop(state);  // 释放锁

        if let Err(e) = ebpf_loader.add_pid_to_whitelist(p).await {
            log::warn!("Failed to add PID {} to eBPF whitelist: {}", p, e);
        } else {
            log::info!("✓ Added PID {} to eBPF monitoring", p);
        }

        // 这里不需要重新获取锁，因为后面没有再使用 state
    }

    log::info!("=== Registration Complete ===");

    HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": format!("Process '{}' registered", req.name),
        "pid": pid,
        "is_running": is_running,
        "stats": stats
    }))
}

pub async fn unregister_process(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let mut state = data.lock().unwrap();
    let name = path.into_inner();

    match state.processes.remove(&name) {
        Some(process_status) => {
            // *** 从 eBPF 白名单移除 ***
            if let Some(pid) = process_status.pid {
                let ebpf_loader = state.ebpf_loader.clone();
                drop(state);

                if let Err(e) = ebpf_loader.remove_pid_from_whitelist(pid).await {
                    log::warn!("Failed to remove PID {} from eBPF whitelist: {}", pid, e);
                } else {
                    log::info!("✓ Removed PID {} from eBPF monitoring", pid);
                }
            }

            HttpResponse::Ok().json(serde_json::json!({
                "status": "success",
                "message": format!("Process '{}' unregistered", name)
            }))
        }
        None => HttpResponse::NotFound().json(serde_json::json!({
            "status": "error",
            "message": format!("Process '{}' not found", name)
        })),
    }
}

pub async fn list_processes(data: web::Data<AppState>) -> impl Responder {
    let state = data.lock().unwrap();
    let list: Vec<_> = state.processes.values().map(|p| {
        serde_json::json!({
            "name": p.config.name,
            "cmdline": p.config.cmdline,
            "labels": p.config.labels,
            "is_running": p.is_running,
            "pid": p.pid,
            "registered_at": p.registered_at,
            "last_check": p.last_check,
            "stats": p.stats
        })
    }).collect();

    HttpResponse::Ok().json(list)
}
