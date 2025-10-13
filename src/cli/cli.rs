use clap::{{Parser}};

/// Process Exporter - 动态进程监控 exporter
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CommandArgs {
    /// 监听端口
    #[arg(short, long, env = "PORT",default_value_t = 9999)]
    pub port: u16,

    /// 监听地址
    #[arg(short = 'a', long, env = "ADDRESS",default_value = "0.0.0.0")]
    pub address: String,
}