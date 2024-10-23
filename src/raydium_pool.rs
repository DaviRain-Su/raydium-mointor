use serde_json::Value;

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
