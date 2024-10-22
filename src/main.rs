use reqwest;
use serde_json::Value;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

pub mod utils;

#[derive(Debug)]
struct PoolInfo {
    id: String,
    symbol_a: String,
    symbol_b: String,
    volume_24h: f64,
    tvl: f64,
}

// 定义监控项结构
struct MonitorItem {
    name: String,
    check_interval: Duration,
    check_fn: fn() -> Result<String, String>,
}

// 定义监控服务
struct MonitorService {
    items: Vec<MonitorItem>,
}

impl MonitorService {
    fn new() -> Self {
        MonitorService { items: Vec::new() }
    }

    fn add_item(
        &mut self,
        name: &str,
        interval: Duration,
        check_fn: fn() -> Result<String, String>,
    ) {
        self.items.push(MonitorItem {
            name: name.to_string(),
            check_interval: interval,
            check_fn,
        });
    }

    fn run(&self) {
        let (tx, rx) = mpsc::channel();

        for item in &self.items {
            let item_name = item.name.clone();
            let item_interval = item.check_interval;
            let item_check = item.check_fn;
            let tx = tx.clone();

            thread::spawn(move || loop {
                let start = Instant::now();
                let result = (item_check)();
                tx.send((item_name.clone(), result)).unwrap();
                let elapsed = start.elapsed();
                if elapsed < item_interval {
                    thread::sleep(item_interval - elapsed);
                }
            });
        }

        for (item_name, result) in rx {
            match result {
                Ok(message) => println!("Item: {}, Status: OK! \n{}", item_name, message),
                Err(error) => println!("Item: {}, Status: FAIL, Error: {}", item_name, error),
            }
        }
    }
}

async fn fetch_raydium_data() -> Result<Value, reqwest::Error> {
    let url = "https://api-v3.raydium.io/pools/info/list?poolType=all&poolSortField=volume24h&sortType=desc&pageSize=100&page=1";
    let response = reqwest::get(url).await?;
    let json: Value = response.json().await?;
    Ok(json)
}

fn check_raydium_pools() -> Result<String, String> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let data = runtime
        .block_on(fetch_raydium_data())
        .map_err(|e| e.to_string())?;

    if let Some(pools) = data["data"]["data"].as_array() {
        let mut pool_infos: Vec<PoolInfo> = pools
            .iter()
            .filter_map(|pool| {
                let id = pool["id"].as_str()?;
                let symbol_a = pool["mintA"]["symbol"].as_str()?;
                let symbol_b = pool["mintB"]["symbol"].as_str()?;
                let volume_24h = pool["day"]["volume"].as_f64()?;
                let tvl = pool["tvl"].as_f64()?;

                // Filter out WSOL/USDC, WSOL/USDT, and WSOL/mSOL pools
                if (symbol_a == "WSOL"
                    && (symbol_b == "USDC" || symbol_b == "USDT" || symbol_b == "mSOL"))
                    || (symbol_b == "WSOL"
                        && (symbol_a == "USDC" || symbol_a == "USDT" || symbol_a == "mSOL"))
                {
                    return None;
                }
                Some(PoolInfo {
                    id: id.to_string(),
                    symbol_a: symbol_a.to_string(),
                    symbol_b: symbol_b.to_string(),
                    volume_24h,
                    tvl,
                })
            })
            .collect();

        // Sort pools by 24h volume in descending order
        pool_infos.sort_by(|a, b| b.volume_24h.partial_cmp(&a.volume_24h).unwrap());

        // Get top 10 pools
        let top_10 = pool_infos.into_iter().take(10).collect::<Vec<_>>();

        let mut result = String::new();
        for (index, pool) in top_10.iter().enumerate() {
            result.push_str(&format!(
                "{}. Pool: {} ({}/{}), 24h Volume: ${:.2}, TVL: ${:.2}\n",
                index + 1,
                pool.id,
                pool.symbol_a,
                pool.symbol_b,
                pool.volume_24h,
                pool.tvl
            ));
        }

        Ok(result)
    } else {
        Err("Failed to parse pool data".to_string())
    }
}

fn main() {
    let mut monitor = MonitorService::new();

    // 添加 Raydium 池子监控
    monitor.add_item(
        "Raydium Top 10 Pools",
        Duration::from_secs(300),
        check_raydium_pools,
    );

    // 运行监控服务
    monitor.run();
}
