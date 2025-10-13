use sysinfo::{System, ProcessesToUpdate, Pid};
use regex::Regex;

/// 检查进程是否正在运行
pub fn check_process_running(cmdline: &str) -> bool {
    get_process_pid(cmdline).is_some()
}

/// 获取进程的 PID（优先返回主进程）
///
/// 对于多线程应用（如 Java），会返回主进程的 PID
///
/// 策略优先级：
/// 1. PPID = 1 的进程（systemd 直接启动）
/// 2. PPID 不在匹配列表中的进程（父进程，非子线程）
/// 3. 最小 PID（通常是最早创建的主进程）
pub fn get_process_pid(cmdline: &str) -> Option<i32> {
    let mut sys = System::new_all();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    let regex = match Regex::new(cmdline) {
        Ok(r) => r,
        Err(_) => return find_main_process_by_string(&sys, cmdline),
    };

    let mut matching_processes = Vec::new();

    for (pid, process) in sys.processes().iter() {
        let process_cmd = process
            .cmd()
            .iter()
            .map(|s| s.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");

        if regex.is_match(&process_cmd) {
            matching_processes.push((
                pid.as_u32() as i32,
                process.parent().map(|p| p.as_u32() as i32),
                process_cmd.clone(),
            ));
        }
    }

    if matching_processes.is_empty() {
        return None;
    }

    if matching_processes.len() == 1 {
        return Some(matching_processes[0].0);
    }

    // 多个匹配时，找主进程

    // 策略1: 找 PPID = 1 的进程（由 systemd 直接启动）
    for (pid, ppid, _) in &matching_processes {
        if let Some(parent_pid) = ppid {
            if *parent_pid == 1 {
                log::debug!("Found main process (PPID=1): PID {}", pid);
                return Some(*pid);
            }
        }
    }

    // 策略2: 找父进程不在匹配列表中的进程（真正的主进程）
    // 例如：docker 启动的进程，PPID 是 dockerd，不在 java 进程列表中
    let matching_pids: Vec<i32> = matching_processes.iter().map(|(pid, _, _)| *pid).collect();

    for (pid, ppid, cmd) in &matching_processes {
        if let Some(parent_pid) = ppid {
            // 如果父进程不在匹配列表中，说明这是主进程
            if !matching_pids.contains(parent_pid) {
                log::debug!("Found main process (parent not in group): PID {} (PPID={})", pid, parent_pid);
                log::debug!("  CMD: {}", cmd);
                return Some(*pid);
            }
        }
    }

    // 策略3: 返回最小的 PID（通常是最早创建的进程）
    let min_pid = matching_processes.iter()
        .map(|(pid, _, _)| *pid)
        .min()
        .unwrap();

    log::debug!("Found main process (min PID fallback): PID {} from {} matches",
                min_pid, matching_processes.len());

    Some(min_pid)
}

/// 使用字符串包含匹配（当正则表达式无效时的后备方案）
fn find_main_process_by_string(sys: &System, pattern: &str) -> Option<i32> {
    let mut matching_processes = Vec::new();

    for (pid, process) in sys.processes() {
        let process_cmd = process
            .cmd()
            .iter()
            .map(|s| s.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        let process_name = process.name().to_string_lossy();

        if process_cmd.contains(pattern) || process_name.contains(pattern) {
            matching_processes.push((
                pid.as_u32() as i32,
                process.parent().map(|p| p.as_u32() as i32),
            ));
        }
    }

    if matching_processes.is_empty() {
        return None;
    }

    if matching_processes.len() == 1 {
        return Some(matching_processes[0].0);
    }

    // 多个匹配时，找主进程
    for (pid, ppid) in &matching_processes {
        if let Some(parent_pid) = ppid {
            if *parent_pid == 1 {
                return Some(*pid);
            }
        }
    }

    // 返回最小 PID
    matching_processes.iter()
        .map(|(pid, _)| *pid)
        .min()
}

/// 获取所有匹配的进程 PIDs
pub fn get_all_matching_pids(cmdline: &str) -> Vec<i32> {
    let mut sys = System::new_all();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    let mut pids = Vec::new();

    let regex = match Regex::new(cmdline) {
        Ok(r) => r,
        Err(_) => {
            // 字符串匹配
            for (pid, process) in sys.processes() {
                let process_cmd = process
                    .cmd()
                    .iter()
                    .map(|s| s.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ");
                let process_name = process.name().to_string_lossy();

                if process_cmd.contains(cmdline) || process_name.contains(cmdline) {
                    pids.push(pid.as_u32() as i32);
                }
            }
            pids.sort();
            return pids;
        }
    };

    // 正则表达式匹配
    for (pid, process) in sys.processes() {
        let process_cmd = process
            .cmd()
            .iter()
            .map(|s| s.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");

        if regex.is_match(&process_cmd) {
            pids.push(pid.as_u32() as i32);
        }
    }

    pids.sort();
    pids
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process;

    #[test]
    fn test_find_current_process() {
        let current_pid = process::id() as i32;
        let found = check_process_running("cargo");
        println!("Found cargo process: {}", found);
    }

    #[test]
    fn test_regex_matching() {
        let pids = get_all_matching_pids("rust.*");
        println!("Found {} rust-related processes", pids.len());
    }
}