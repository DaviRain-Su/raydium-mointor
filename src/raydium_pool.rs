use crate::utils::calculate_market_cap;
use futures::future;
use serde_json::Value;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
struct PoolInfo {
    id: String,
    symbol_a: String,
    symbol_b: String,
    volume_24h: f64,
    tvl: f64,
}

async fn fetch_raydium_data(page: u32) -> anyhow::Result<Value> {
    let url = format!(
        "https://api-v3.raydium.io/pools/info/list?poolType=all&poolSortField=volume24h&sortType=desc&pageSize=100&page={}",
        page
    );
    let response = reqwest::get(&url).await?;
    let json: Value = response.json().await?;
    Ok(json)
}

pub async fn check_raydium_pools() -> anyhow::Result<String> {
    let data = fetch_raydium_data(1).await?;

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
        let top_10 = pool_infos.into_iter().take(20).collect::<Vec<_>>();

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
        Err(anyhow::anyhow!("Failed to parse pool data"))
    }
}

pub async fn filter_tokens(
    min_ratio: f64,
    max_ratio: f64,
    limit: usize,
    verbose: bool,
) -> anyhow::Result<()> {
    let all_filtered_tokens = Arc::new(Mutex::new(Vec::new()));
    let mut page = 1;
    let concurrency_limit = 10; // 可以根据需要调整并发数

    loop {
        let mut futures = Vec::new();
        for _ in 0..concurrency_limit {
            let current_page = page;
            let all_filtered_tokens = Arc::clone(&all_filtered_tokens);
            let future = tokio::spawn(async move {
                let result =
                    process_page(current_page, min_ratio, max_ratio, all_filtered_tokens).await;
                result
            });
            futures.push(future);
            page += 1;
        }

        let results = future::join_all(futures).await;
        if results
            .into_iter()
            .any(|r| r.unwrap_or(Ok(true)).unwrap_or(true))
        {
            break;
        }

        println!("Processed up to page {}", page - 1);
    }

    let mut all_filtered_tokens = Arc::try_unwrap(all_filtered_tokens).unwrap().into_inner();
    all_filtered_tokens.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

    println!(
        "Tokens with volume/marketcap ratio between {} and {}:",
        min_ratio, max_ratio
    );
    for (token, token_address, ratio, volume, market_cap) in all_filtered_tokens.iter().take(limit)
    {
        if verbose {
            println!(
                "{} - ({}): Ratio: {:.4}, Volume: ${:.2}, Market Cap: ${:.2}",
                token, token_address, ratio, volume, market_cap
            );
        } else {
            println!("{}: {:.4}", token, ratio);
        }
    }

    Ok(())
}

async fn process_page(
    page: u32,
    min_ratio: f64,
    max_ratio: f64,
    all_filtered_tokens: Arc<Mutex<Vec<(String, String, f64, f64, f64)>>>,
) -> anyhow::Result<bool> {
    if let Ok(json) = fetch_raydium_data(page).await {
        if let Some(pools) = json["data"]["data"].as_array() {
            for pool in pools {
                if let (Some(symbol), Some(token_address), Some(day_volume)) = (
                    pool["mintB"]["symbol"].as_str(),
                    pool["mintB"]["address"].as_str(),
                    pool["day"]["volume"].as_f64(),
                ) {
                    if let Ok(market_cap) = calculate_market_cap(pool).await {
                        let volume_marketcap_ratio = day_volume / market_cap;
                        if volume_marketcap_ratio >= min_ratio
                            && volume_marketcap_ratio <= max_ratio
                        {
                            log::info!("{}: {:.4}", symbol, volume_marketcap_ratio);
                            let mut tokens = all_filtered_tokens.lock().await;
                            tokens.push((
                                symbol.to_string(),
                                token_address.to_string(),
                                volume_marketcap_ratio,
                                day_volume,
                                market_cap,
                            ));
                        }
                    }
                }
            }
            return Ok(pools.is_empty());
        }
    }
    Ok(true) // 如果出现错误，我们假设页面为空
}
