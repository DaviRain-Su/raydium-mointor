use crate::mointor::MonitorService;
use crate::mointor::MonitorStatus;
use crate::raydium_pool::check_raydium_pools;
use crate::raydium_pool::filter_tokens;
use crate::raydium_pool::token_info;
use crate::raydium_pool::top_volume;
use log::{error, info, warn, LevelFilter};

use std::error::Error;
use std::time::Duration;
use structopt::StructOpt;
pub mod mointor;
pub mod raydium_pool;
pub mod utils;

#[derive(StructOpt, Debug)]
#[structopt(name = "raydium_tool")]
pub enum Command {
    /// Filter tokens based on volume/marketcap ratio
    Filter {
        /// Minimum volume/marketcap ratio
        #[structopt(long, default_value = "0.8")]
        min_ratio: f64,

        /// Maximum volume/marketcap ratio
        #[structopt(long, default_value = "1.0")]
        max_ratio: f64,

        /// Number of results to display
        #[structopt(short, long, default_value = "10")]
        limit: usize,

        /// Display detailed information
        #[structopt(short, long)]
        verbose: bool,
    },

    /// List top tokens by volume
    TopVolume {
        /// Number of tokens to display
        #[structopt(short, long, default_value = "10")]
        limit: usize,
    },

    /// Show details for a specific token
    TokenInfo {
        /// Token symbol
        #[structopt(name = "SYMBOL")]
        symbol: String,
    },
    /// Run the monitoring service
    Monitor {
        /// Interval in seconds for checking Raydium pools
        #[structopt(short, long, default_value = "300")]
        interval: u64,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 初始化日志系统
    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .init();

    let command = Command::from_args();

    match command {
        Command::Filter {
            min_ratio,
            max_ratio,
            limit,
            verbose,
        } => {
            filter_tokens(min_ratio, max_ratio, limit, verbose).await?;
        }
        Command::TopVolume { limit } => {
            top_volume(limit).await?;
        }
        Command::TokenInfo { symbol } => {
            token_info(&symbol).await?;
        }
        Command::Monitor { interval: _ } => {
            info!("Starting monitoring service...");

            let mut monitor = MonitorService::new();

            // 添加内存监控
            monitor
                .add_item("raydium pool", Duration::from_secs(5), || async {
                    check_raydium_pools().await
                })
                .await;

            // 启动监控
            monitor.run().await?;

            // 订阅监控事件
            let mut rx = monitor.tx.subscribe();
            tokio::spawn(async move {
                while let Ok(event) = rx.recv().await {
                    match &event.status {
                        MonitorStatus::OK(msg) => info!("{}: {}", event.item_name, msg),
                        MonitorStatus::Warning(msg) => warn!("{}: {}", event.item_name, msg),
                        MonitorStatus::Error(err) => error!("{}: {}", event.item_name, err),
                    }
                }
            });

            // 运行一段时间
            info!("Monitor service running...");
            tokio::time::sleep(Duration::from_secs(30)).await;

            // 停止服务
            monitor.stop().await;
        }
    }

    Ok(())
}
