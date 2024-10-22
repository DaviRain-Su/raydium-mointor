use crate::mointor::MonitorService;
use crate::raydium_pool::check_raydium_pools;
use crate::raydium_pool::filter_tokens;
use crate::raydium_pool::token_info;
use crate::raydium_pool::top_volume;
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
    env_logger::init();

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
        Command::Monitor { interval } => {
            let mut monitor = MonitorService::new();
            monitor.add_item(
                "Raydium Top 10 Pools",
                Duration::from_secs(interval),
                check_raydium_pools,
            );
            monitor.run();
        }
    }

    Ok(())
}
