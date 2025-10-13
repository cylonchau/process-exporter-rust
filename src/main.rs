use actix_web::{web, App, HttpServer};
use clap::Parser;

mod models;
mod services;
mod state;
mod api;
mod cli;
mod metrics;

use state::new_state;
use api::{register_process, unregister_process, list_processes, get_metrics, health};
use cli::CommandArgs;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = CommandArgs::parse();
    let bind_address = format!("{}:{}", args.address, args.port);

    let state = new_state();

    // 加载 eBPF
    let ebpf_loader = {
        let state_guard = state.lock().unwrap();
        state_guard.ebpf_loader.clone()
    };

    log::info!("🔄 Loading eBPF program...");

    match ebpf_loader.load().await {
        Ok(_) => {
            log::info!("✅ eBPF network monitoring loaded successfully");
        }
        Err(e) => {
            log::error!("❌ Failed to load eBPF program: {}", e);
            log::error!("   Full error chain:");
            let mut current_error: Option<&dyn std::error::Error> = Some(e.as_ref());
            while let Some(err) = current_error {
                log::error!("     - {}", err);
                current_error = err.source();
            }
            log::warn!("   Network traffic monitoring will be disabled");
            log::warn!("   Tip: Run with 'sudo' for eBPF support");

            #[cfg(debug_assertions)]
            log::error!("   Expected eBPF file: ebpf/target/bpfel-unknown-none/debug/network-monitor");
            #[cfg(not(debug_assertions))]
            log::error!("   Expected eBPF file: ebpf/target/bpfel-unknown-none/release/network-monitor");

            // 检查文件是否存在
            #[cfg(debug_assertions)]
            let ebpf_path = "ebpf/target/bpfel-unknown-none/debug/network-monitor";
            #[cfg(not(debug_assertions))]
            let ebpf_path = "ebpf/target/bpfel-unknown-none/release/network-monitor";

            if std::path::Path::new(ebpf_path).exists() {
                log::error!("   ✓ eBPF file exists at: {}", ebpf_path);
            } else {
                log::error!("   ✗ eBPF file NOT found at: {}", ebpf_path);
                log::error!("   Please run: cd ebpf && cargo +nightly build --release");
            }
        }
    }

    print_banner(&args);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .route("/api/process/register", web::post().to(register_process))
            .route("/api/process/{name}", web::delete().to(unregister_process))
            .route("/api/process/list", web::get().to(list_processes))
            .route("/metrics", web::get().to(get_metrics))
            .route("/health", web::get().to(health))
    })
        .bind(&bind_address)?
        .run()
        .await
}

fn print_banner(args: &CommandArgs) {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║      Process Exporter v0.1.1                              ║");
    println!("║      With eBPF Network Monitoring                         ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();
    println!("🚀 Server starting on http://{}:{}", args.address, args.port);
    println!();
    println!("📋 Available endpoints:");
    println!("  POST   /api/process/register   - Register a process");
    println!("  DELETE /api/process/{{name}}     - Unregister a process");
    println!("  GET    /api/process/list       - List all processes");
    println!("  GET    /metrics                - Prometheus metrics");
    println!("  GET    /health                 - Health check");
    println!();
    println!("💡 Features:");
    println!("  • CPU, Memory, Disk monitoring (sysinfo)");
    println!("  • Network traffic monitoring (eBPF)");
    println!("  • Prometheus metrics export");
    println!("═══════════════════════════════════════════════════════════");
}
