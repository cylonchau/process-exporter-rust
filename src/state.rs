use crate::models::ProcessStatus;
use crate::services::{StatsCollector, ebpf_loader::EbpfLoader};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct AppStateInner {
    pub processes: HashMap<String, ProcessStatus>,
    pub stats_collector: StatsCollector,
    pub ebpf_loader: Arc<EbpfLoader>,
}

pub type AppState = Arc<Mutex<AppStateInner>>;

pub fn new_state() -> AppState {
    let ebpf_loader = Arc::new(EbpfLoader::new());
    
    Arc::new(Mutex::new(AppStateInner {
        processes: HashMap::new(),
        stats_collector: StatsCollector::new(ebpf_loader.clone()),  // ← 传递 ebpf_loader
        ebpf_loader,
    }))
}
