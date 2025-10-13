use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct ProcessStats {
    /// CPU 使用率 (百分比，0-100)
    pub cpu_usage: f32,

    /// 内存使用量 (字节)
    pub memory_bytes: u64,

    /// 内存使用率 (百分比，0-100)
    pub memory_percent: f32,

    /// 虚拟内存使用量 (字节)
    pub virtual_memory_bytes: u64,

    /// 磁盘读取字节数
    pub disk_read_bytes: u64,

    /// 磁盘写入字节数
    pub disk_written_bytes: u64,

    /// 线程数
    pub thread_count: usize,

    // ebpf相关状态
    pub network_tx_bytes: u64,
    pub network_rx_bytes: u64,
    pub network_tx_packets: u64,
    pub network_rx_packets: u64,
}

impl ProcessStats {
    /// 创建一个空的统计数据
    pub fn empty() -> Self {
        Self::default()
    }

    /// 判断是否有有效数据
    pub fn is_valid(&self) -> bool {
        self.cpu_usage > 0.0 || self.memory_bytes > 0
    }
}