pub mod mointor;
pub mod raydium_pool;
pub mod utils;

use log::LevelFilter;
use std::error::Error;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "raydium_tool")]
pub enum Command {
    Monitor {
        /// 检查间隔（秒）
        #[structopt(short, long, default_value = "30")]
        interval: u64,

        /// 显示前N个池子
        #[structopt(short, long, default_value = "20")]
        top_n: usize,

        /// 价格变化警报阈值(%)
        #[structopt(long, default_value = "1.0")]
        price_alert: f64,

        /// 交易量变化警报阈值(%)
        #[structopt(long, default_value = "5.0")]
        volume_alert: f64,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .init();

    let command = Command::from_args();

    match command {
        Command::Monitor {
            interval: _,
            top_n: _,
            price_alert: _,
            volume_alert: _,
        } => {}
    }
    Ok(())
}
