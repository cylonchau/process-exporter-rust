use serde::{Deserialize, Serialize};
use std::collections::HashMap;
pub use crate::models::stats::ProcessStats;

/// 进程配置信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessConfig {
    /// 进程名称（唯一标识符）
    pub name: String,
    /// 用于匹配进程的命令行模式
    pub cmdline: String,
    /// 自定义标签
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

/// 进程运行状态
#[derive(Debug, Clone)]
pub struct ProcessStatus {
    /// 进程配置
    pub config: ProcessConfig,
    /// 注册时间戳（Unix 时间）
    pub registered_at: u64,
    /// 最后检查时间戳
    pub last_check: u64,
    /// 是否正在运行
    pub is_running: bool,
    /// 进程 ID
    pub pid: Option<i32>,
    /// 进程资源使用统计
    pub stats: ProcessStats,
}