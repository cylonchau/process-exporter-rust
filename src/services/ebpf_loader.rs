use aya::{
    include_bytes_aligned,
    maps::HashMap as AyaHashMap,
    programs::KProbe,
    Ebpf,
};
use aya_log::EbpfLogger;
use std::sync::Arc;
use tokio::sync::Mutex;
use log::{info, warn};

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NetworkStats {
    pub tx_bytes: u64,
    pub rx_bytes: u64,
    pub tx_packets: u64,
    pub rx_packets: u64,
}

unsafe impl aya::Pod for NetworkStats {}

pub struct EbpfLoader {
    ebpf: Arc<Mutex<Option<Ebpf>>>,
}

impl EbpfLoader {
    pub fn new() -> Self {
        Self {
            ebpf: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn load(&self) -> anyhow::Result<()> {
        // 使用编译器建议的路径
        #[cfg(debug_assertions)]
        let ebpf_data = include_bytes_aligned!("../../ebpf/target/bpfel-unknown-none/debug/network-monitor");
        #[cfg(not(debug_assertions))]
        let ebpf_data = include_bytes_aligned!("../../ebpf/target/bpfel-unknown-none/release/network-monitor");

        let mut ebpf = Ebpf::load(ebpf_data)
            .map_err(|e| anyhow::anyhow!("Failed to load eBPF: {:?}", e))?;

        if let Err(e) = EbpfLogger::init(&mut ebpf) {
            warn!("Failed to init eBPF logger: {}", e);
        } else {
            info!("✓ eBPF logger initialized");
        }

        // 列出所有可用的程序以便调试
        info!("Available eBPF programs:");
        for (name, _) in ebpf.programs() {
            info!("  - {}", name);
        }

        // 附加 tcp_sendmsg kretprobe
        info!("Attaching tcp_sendmsg kretprobe...");
        let program: &mut KProbe = ebpf
            .program_mut("tcp_sendmsg")
            .ok_or_else(|| anyhow::anyhow!("tcp_sendmsg program not found"))?
            .try_into()
            .map_err(|e| anyhow::anyhow!("Failed to convert to KProbe: {:?}", e))?;

        program.load()
            .map_err(|e| anyhow::anyhow!("Failed to load tcp_sendmsg: {:?}", e))?;
        
        program.attach("tcp_sendmsg", 0)
            .map_err(|e| anyhow::anyhow!("Failed to attach tcp_sendmsg: {:?}", e))?;
        info!("✓ Attached kretprobe: tcp_sendmsg");

        // 附加 tcp_recvmsg kretprobe
        info!("Attaching tcp_recvmsg kretprobe...");
        let program: &mut KProbe = ebpf
            .program_mut("tcp_recvmsg")
            .ok_or_else(|| anyhow::anyhow!("tcp_recvmsg program not found"))?
            .try_into()
            .map_err(|e| anyhow::anyhow!("Failed to convert to KProbe: {:?}", e))?;

        program.load()
            .map_err(|e| anyhow::anyhow!("Failed to load tcp_recvmsg: {:?}", e))?;
        
        program.attach("tcp_recvmsg", 0)
            .map_err(|e| anyhow::anyhow!("Failed to attach tcp_recvmsg: {:?}", e))?;
        info!("✓ Attached kretprobe: tcp_recvmsg");

        *self.ebpf.lock().await = Some(ebpf);

        info!("🎉 All eBPF programs loaded and attached successfully");
        Ok(())
    }

    pub async fn add_pid_to_whitelist(&self, pid: i32) -> anyhow::Result<()> {
        let mut ebpf_guard = self.ebpf.lock().await;
        let ebpf = ebpf_guard.as_mut()
            .ok_or_else(|| anyhow::anyhow!("eBPF not loaded"))?;

        let mut whitelist: AyaHashMap<_, u32, u8> = AyaHashMap::try_from(
            ebpf.map_mut("PID_WHITELIST")
                .ok_or_else(|| anyhow::anyhow!("PID_WHITELIST map not found"))?
        ).map_err(|e| anyhow::anyhow!("Failed to get whitelist map: {:?}", e))?;

        whitelist.insert(pid as u32, 1, 0)
            .map_err(|e| anyhow::anyhow!("Failed to add PID to whitelist: {:?}", e))?;

        info!("✓ Added PID {} to eBPF whitelist", pid);
        Ok(())
    }

    pub async fn remove_pid_from_whitelist(&self, pid: i32) -> anyhow::Result<()> {
        let mut ebpf_guard = self.ebpf.lock().await;
        let ebpf = ebpf_guard.as_mut()
            .ok_or_else(|| anyhow::anyhow!("eBPF not loaded"))?;

        let mut whitelist: AyaHashMap<_, u32, u8> = AyaHashMap::try_from(
            ebpf.map_mut("PID_WHITELIST")
                .ok_or_else(|| anyhow::anyhow!("PID_WHITELIST map not found"))?
        ).map_err(|e| anyhow::anyhow!("Failed to get whitelist map: {:?}", e))?;

        whitelist.remove(&(pid as u32))
            .map_err(|e| anyhow::anyhow!("Failed to remove PID from whitelist: {:?}", e))?;

        info!("✓ Removed PID {} from eBPF whitelist", pid);
        Ok(())
    }

    pub async fn get_network_stats(&self, pid: i32) -> Option<NetworkStats> {
        let ebpf_guard = self.ebpf.lock().await;
        let ebpf = ebpf_guard.as_ref()?;

        let network_stats: AyaHashMap<_, u32, NetworkStats> = AyaHashMap::try_from(
            ebpf.map("NETWORK_STATS")?
        ).ok()?;

        network_stats.get(&(pid as u32), 0).ok()
    }

    pub async fn get_all_stats(&self) -> Vec<(u32, NetworkStats)> {
        let ebpf_guard = self.ebpf.lock().await;
        let Some(ebpf) = ebpf_guard.as_ref() else {
            return Vec::new();
        };

        let Some(map) = ebpf.map("NETWORK_STATS") else {
            return Vec::new();
        };

        let Ok(network_stats) = AyaHashMap::<_, u32, NetworkStats>::try_from(map) else {
            return Vec::new();
        };

        network_stats
            .iter()
            .filter_map(|item| item.ok())
            .collect()
    }
}

impl Default for EbpfLoader {
    fn default() -> Self {
        Self::new()
    }
}
