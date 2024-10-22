use serde_json::Value;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub async fn get_sol_price() -> anyhow::Result<f64> {
    let url =
        "https://api-v3.raydium.io/pools/info/ids?ids=8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj";
    let response = reqwest::get(url).await?.text().await?;
    let json: Value = serde_json::from_str(&response)?;

    // 从JSON中提取价格
    let price = json["data"][0]["price"]
        .as_f64()
        .ok_or(anyhow::anyhow!("Failed to extract price from JSON"))?;

    Ok(price)
}

pub async fn get_token_supply(token_address: &str) -> anyhow::Result<u64> {
    let rpc_url = "https://api.mainnet-beta.solana.com";
    let client = RpcClient::new(rpc_url.to_string());

    let token_pubkey = Pubkey::from_str(token_address)?;
    let supply = client.get_token_supply(&token_pubkey)?;
    log::debug!("SUPPLY: {:?}", supply);
    Ok(supply.amount.parse().unwrap())
}

pub async fn calculate_market_cap(token_data: &serde_json::Value) -> anyhow::Result<f64> {
    let token_address = token_data["mintB"]["address"].as_str().unwrap();
    let token_decimals = token_data["mintB"]["decimals"].as_u64().unwrap();
    let price_in_sol = 1.0 / token_data["price"].as_f64().unwrap();

    // 获取 SOL 价格（以 USDC 计）
    let sol_price = get_sol_price().await?;

    // 将 SOL 价格转换为 USDC 价格
    let price_in_usdc = price_in_sol * sol_price;

    let total_supply = get_token_supply(token_address).await?;
    let total_supply_adjusted = total_supply as f64 / 10f64.powi(token_decimals as i32);

    let market_cap = total_supply_adjusted * price_in_usdc;

    Ok(market_cap)
}

#[test]
fn test_market_cap() -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    // 假设token_data是您之前提供的API返回的JSON数据
    let str = r#"{
            "type": "Standard",
            "programId": "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
            "id": "6QVQKPE5JeWTwsSumYJkJHPHoukW23D8XeRLzk7oAnqg",
            "mintA": {
              "chainId": 101,
              "address": "So11111111111111111111111111111111111111112",
              "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
              "logoURI": "https://img-v1.raydium.io/icon/So11111111111111111111111111111111111111112.png",
              "symbol": "WSOL",
              "name": "Wrapped SOL",
              "decimals": 9,
              "tags": [],
              "extensions": {}
            },
            "mintB": {
              "chainId": 101,
              "address": "FqvtZ2UFR9we82Ni4LeacC1zyTiQ77usDo31DUokpump",
              "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
              "logoURI": "https://img-v1.raydium.io/icon/FqvtZ2UFR9we82Ni4LeacC1zyTiQ77usDo31DUokpump.png",
              "symbol": "$slop",
              "name": "slop",
              "decimals": 6,
              "tags": [],
              "extensions": {}
            },
            "price": 6948.933948075416,
            "mintAmountA": 3535.924424012,
            "mintAmountB": 24570905.267846,
            "feeRate": 0.0025,
            "openTime": "0",
            "tvl": 1171602.1,
            "day": {
              "volume": 152266185.89469922,
              "volumeQuote": 9919951763.189098,
              "volumeFee": 380665.46473674703,
              "apr": 11859.22,
              "feeApr": 11859.22,
              "priceMin": 3044.857707702913,
              "priceMax": 245156.5042684039,
              "rewardApr": []
            },
            "week": {
              "volume": 179906535.6488955,
              "volumeQuote": 16623931467.00907,
              "volumeFee": 449766.3391222376,
              "apr": 1151.67,
              "feeApr": 1151.67,
              "priceMin": 3044.857707702913,
              "priceMax": 2630176.7652169303,
              "rewardApr": []
            },
            "month": {
              "volume": 179906535.6488955,
              "volumeQuote": 16623931467.00907,
              "volumeFee": 449766.3391222376,
              "apr": 460.67,
              "feeApr": 460.67,
              "priceMin": 3044.857707702913,
              "priceMax": 2630176.7652169303,
              "rewardApr": []
            },
            "pooltype": [
              "OpenBookMarket"
            ],
            "rewardDefaultInfos": [],
            "farmUpcomingCount": 0,
            "farmOngoingCount": 0,
            "farmFinishedCount": 0,
            "marketId": "H1wKFpzr7aXXQP6zVgMVbZSUwEXvSjr9vT7DCwEUqULg",
            "lpMint": {
              "chainId": 101,
              "address": "HThpmCrwsn7bueJaFa5ScrU9HtA9ToaMVU1rSccXUsjG",
              "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
              "logoURI": "",
              "symbol": "",
              "name": "",
              "decimals": 9,
              "tags": [],
              "extensions": {}
            },
            "lpPrice": 233.13939870731943,
            "lpAmount": 5025.328651732,
            "burnPercent": 80.45
          }"#;

    rt.block_on(async {
        let token_data: serde_json::Value = serde_json::from_str(str).unwrap();
        println!("{:?}", token_data);

        let market_cap = calculate_market_cap(&token_data).await.unwrap();

        println!("Estimated market cap: ${:.2}", market_cap);
    });

    Ok(())
}
