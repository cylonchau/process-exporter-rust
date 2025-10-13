#![no_std]
#![no_main]

use aya_ebpf::{
    macros::{kretprobe, map},
    maps::HashMap,
    programs::RetProbeContext,
    helpers::bpf_get_current_pid_tgid,
};
use aya_log_ebpf::debug;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NetworkStats {
    pub tx_bytes: u64,
    pub rx_bytes: u64,
    pub tx_packets: u64,
    pub rx_packets: u64,
}

#[map]
static NETWORK_STATS: HashMap<u32, NetworkStats> = HashMap::with_max_entries(10240, 0);

#[map]
static PID_WHITELIST: HashMap<u32, u8> = HashMap::with_max_entries(10240, 0);

#[kretprobe]
pub fn tcp_sendmsg(ctx: RetProbeContext) -> u32 {
    match try_tcp_sendmsg(&ctx) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

fn try_tcp_sendmsg(ctx: &RetProbeContext) -> Result<u32, i64> {
    let pid_tgid = bpf_get_current_pid_tgid();
    let tgid = (pid_tgid >> 32) as u32;
    let tid = (pid_tgid & 0xFFFFFFFF) as u32;

    let ret: i64 = ctx.ret().ok_or(0i64)?;
    
    // 严格检查：只接受正数且合理范围的值 (最大 1MB)
    if ret <= 0 || ret > 1048576 {
        return Ok(0);
    }

    let sent_bytes = ret as u64;

    let whitelist_value = unsafe {
        PID_WHITELIST.get(&tgid).copied().unwrap_or(0)
    };

    if whitelist_value == 0 {
        return Ok(0);
    }

    debug!(ctx, "[TX] TGID={} TID={} sent={} bytes", tgid, tid, sent_bytes);

    let stats = unsafe {
        NETWORK_STATS.get(&tgid).copied().unwrap_or(NetworkStats {
            tx_bytes: 0,
            rx_bytes: 0,
            tx_packets: 0,
            rx_packets: 0,
        })
    };

    let new_stats = NetworkStats {
        tx_bytes: stats.tx_bytes.saturating_add(sent_bytes),
        tx_packets: stats.tx_packets.saturating_add(1),
        rx_bytes: stats.rx_bytes,
        rx_packets: stats.rx_packets,
    };

    let _ = unsafe {
        NETWORK_STATS.insert(&tgid, &new_stats, 0)
    };

    Ok(0)
}

#[kretprobe]
pub fn tcp_recvmsg(ctx: RetProbeContext) -> u32 {
    match try_tcp_recvmsg(&ctx) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

fn try_tcp_recvmsg(ctx: &RetProbeContext) -> Result<u32, i64> {
    let pid_tgid = bpf_get_current_pid_tgid();
    let tgid = (pid_tgid >> 32) as u32;
    let tid = (pid_tgid & 0xFFFFFFFF) as u32;

    let ret: i64 = ctx.ret().ok_or(0i64)?;
    
    // 严格检查：只接受正数且小于 1MB 的单次接收
    if ret <= 0 || ret > 1048576 {
        if ret < 0 {
            // 负数是错误码，完全正常，不记录
        } else if ret > 1048576 {
            // 异常大的值，记录警告
            debug!(ctx, "[WARN] Abnormal recv size: TGID={} ret={}", tgid, ret);
        }
        return Ok(0);
    }

    let recv_bytes = ret as u64;

    let whitelist_value = unsafe {
        PID_WHITELIST.get(&tgid).copied().unwrap_or(0)
    };

    if whitelist_value == 0 {
        return Ok(0);
    }

    debug!(ctx, "[RX] TGID={} TID={} recv={} bytes", tgid, tid, recv_bytes);

    let stats = unsafe {
        NETWORK_STATS.get(&tgid).copied().unwrap_or(NetworkStats {
            tx_bytes: 0,
            rx_bytes: 0,
            tx_packets: 0,
            rx_packets: 0,
        })
    };

    let new_stats = NetworkStats {
        tx_bytes: stats.tx_bytes,
        tx_packets: stats.tx_packets,
        rx_bytes: stats.rx_bytes.saturating_add(recv_bytes),
        rx_packets: stats.rx_packets.saturating_add(1),
    };

    let _ = unsafe {
        NETWORK_STATS.insert(&tgid, &new_stats, 0)
    };

    Ok(0)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
