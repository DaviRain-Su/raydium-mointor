use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

// 扩展池信息结构体，添加市值字段
#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub id: String,
    pub symbol_a: String,
    pub symbol_a_address: String,
    pub symbol_b: String,
    pub symbol_b_address: String,
    pub symbol_b_decimals: u64,
    pub volume_24h: f64,
    pub tvl: f64,
    pub price: f64,
    pub timestamp: DateTime<Utc>,
}

// 扩展历史数据结构体，添加市值
#[derive(Debug, Clone)]
pub struct HistoricalData {
    pub volume_24h: f64,
    pub price: f64,
    pub tvl: f64,
    pub timestamp: DateTime<Utc>,
}

// 扩展变化指标结构体，添加市值变化
#[derive(Debug)]
pub struct ChangeMetrics {
    pub volume_change_5m: f64,  // 5分钟变化
    pub volume_change_15m: f64, // 15分钟变化
    pub volume_change_1h: f64,  // 1小时变化
    pub volume_change_24h: f64, // 24小时变化
    pub price_change_5m: f64,   // 5分钟变化
    pub price_change_15m: f64,  // 15分钟变化
    pub price_change_1h: f64,   // 1小时变化
    pub price_change_24h: f64,  // 24小时变化
    pub tvl_change_24h: f64,
}

pub struct PoolMonitor {
    pub historical_data: Arc<Mutex<HashMap<String, Vec<HistoricalData>>>>,
    pub last_update: Arc<Mutex<DateTime<Utc>>>,
}

impl PoolMonitor {
    pub fn new() -> Self {
        PoolMonitor {
            historical_data: Arc::new(Mutex::new(HashMap::new())),
            last_update: Arc::new(Mutex::new(Utc::now())),
        }
    }

    // 计算变化率
    pub fn calculate_change(old_value: f64, new_value: f64) -> f64 {
        ((new_value - old_value) / old_value) * 100.0
    }

    // 修改获取变化指标的方法
    pub async fn get_changes(&self, pool_id: &str, _minutes: i64) -> Option<ChangeMetrics> {
        let historical_data = self.historical_data.lock().await;
        let pool_history = historical_data.get(pool_id)?;

        if pool_history.is_empty() {
            return None;
        }

        let latest = pool_history.last()?;

        // 获取不同时间点的历史数据
        let time_5m = latest.timestamp - chrono::Duration::minutes(5);
        let time_15m = latest.timestamp - chrono::Duration::minutes(15);
        let time_1h = latest.timestamp - chrono::Duration::hours(1);
        let time_24h = latest.timestamp - chrono::Duration::hours(24);

        // 查找最接近的历史记录
        let record_5m = pool_history.iter().rev().find(|r| r.timestamp <= time_5m);
        let record_15m = pool_history.iter().rev().find(|r| r.timestamp <= time_15m);
        let record_1h = pool_history.iter().rev().find(|r| r.timestamp <= time_1h);
        let record_24h = pool_history.iter().rev().find(|r| r.timestamp <= time_24h);

        Some(ChangeMetrics {
            volume_change_5m: record_5m
                .map(|r| Self::calculate_change(r.volume_24h, latest.volume_24h))
                .unwrap_or(0.0),
            volume_change_15m: record_15m
                .map(|r| Self::calculate_change(r.volume_24h, latest.volume_24h))
                .unwrap_or(0.0),
            volume_change_1h: record_1h
                .map(|r| Self::calculate_change(r.volume_24h, latest.volume_24h))
                .unwrap_or(0.0),
            volume_change_24h: record_24h
                .map(|r| Self::calculate_change(r.volume_24h, latest.volume_24h))
                .unwrap_or(0.0),
            price_change_5m: record_5m
                .map(|r| Self::calculate_change(r.price, latest.price))
                .unwrap_or(0.0),
            price_change_15m: record_15m
                .map(|r| Self::calculate_change(r.price, latest.price))
                .unwrap_or(0.0),
            price_change_1h: record_1h
                .map(|r| Self::calculate_change(r.price, latest.price))
                .unwrap_or(0.0),
            price_change_24h: record_24h
                .map(|r| Self::calculate_change(r.price, latest.price))
                .unwrap_or(0.0),
            tvl_change_24h: record_24h
                .map(|r| Self::calculate_change(r.tvl, latest.tvl))
                .unwrap_or(0.0),
        })
    }

    // 修正后的更新历史数据方法
    pub async fn update_historical_data(&self, pool_info: &PoolInfo) {
        let mut historical_data = self.historical_data.lock().await;
        let pool_history = historical_data
            .entry(pool_info.id.clone())
            .or_insert_with(Vec::new);

        // 添加新的历史记录，包含市值数据
        pool_history.push(HistoricalData {
            volume_24h: pool_info.volume_24h,
            price: pool_info.price,
            tvl: pool_info.tvl,
            timestamp: pool_info.timestamp,
        });

        // 保留最近7天的数据
        let week_ago = Utc::now() - chrono::Duration::days(7);
        pool_history.retain(|record| record.timestamp > week_ago);

        // 可选：输出调试信息
        log::debug!(
            "Updated historical data for pool {}: {} records stored",
            pool_info.id,
            pool_history.len()
        );
    }
}

pub async fn fetch_raydium_data(page: u32) -> Result<Value> {
    let url = format!(
        "https://api-v3.raydium.io/pools/info/list?poolType=all&poolSortField=volume24h&sortType=desc&pageSize=100&page={}",
        page
    );
    let response = reqwest::get(&url).await?;
    let json: Value = response.json().await?;
    Ok(json)
}

// 首先创建一个用于返回的数据结构
#[derive(Debug, Clone)]
pub struct PoolDataResult {
    pub pools: Vec<PoolInfo>,
    pub timestamp: DateTime<Utc>,
}

pub async fn check_raydium_pools() -> Result<PoolDataResult> {
    let data = fetch_raydium_data(1).await?;
    let current_time = Utc::now();
    log::info!("Checking Raydium pools at {}", current_time);

    if let Some(pools) = data["data"]["data"].as_array() {
        let mut pool_infos: Vec<PoolInfo> = Vec::new();

        for pool in pools {
            if let (
                Some(id),
                Some(symbol_a),
                Some(symbol_b),
                Some(symbol_a_address),
                Some(symbol_b_address),
                Some(symbol_b_decimals),
            ) = (
                pool["id"].as_str(),
                pool["mintA"]["symbol"].as_str(),
                pool["mintB"]["symbol"].as_str(),
                pool["mintA"]["address"].as_str(),
                pool["mintB"]["address"].as_str(),
                pool["mintB"]["decimals"].as_u64(),
            ) {
                // 过滤特定池
                if (symbol_a == "WSOL"
                    && (symbol_b == "USDC" || symbol_b == "USDT" || symbol_b == "mSOL"))
                    || (symbol_b == "WSOL"
                        && (symbol_a == "USDC" || symbol_a == "USDT" || symbol_a == "mSOL"))
                {
                    continue;
                }

                let volume_24h = pool["day"]["volume"].as_f64().unwrap_or(0.0);
                let tvl = pool["tvl"].as_f64().unwrap_or(0.0);
                let price = pool["price"].as_f64().unwrap_or(0.0);

                pool_infos.push(PoolInfo {
                    id: id.to_string(),
                    symbol_a: symbol_a.to_string(),
                    symbol_a_address: symbol_a_address.to_string(),
                    symbol_b: symbol_b.to_string(),
                    symbol_b_address: symbol_b_address.to_string(),
                    symbol_b_decimals,
                    volume_24h,
                    tvl,
                    price,
                    timestamp: current_time,
                });
            }
        }

        // 按24小时交易量排序
        pool_infos.sort_by(|a, b| b.volume_24h.partial_cmp(&a.volume_24h).unwrap());

        Ok(PoolDataResult {
            pools: pool_infos,
            timestamp: current_time,
        })
    } else {
        Err(anyhow::anyhow!("Failed to parse pool data"))
    }
}

// 添加一个格式化函数用于显示
pub async fn format_pool_data(
    pool_data: &PoolDataResult,
    pool_monitor: &PoolMonitor,
    top_n: usize,
    price_alert: f64,
    volume_alert: f64,
) -> String {
    let mut result = String::new();
    result.push_str(&format!(
        "🕒 Update time: {}\n\n",
        pool_data.timestamp.format("%Y-%m-%d %H:%M:%S")
    ));

    for pool_info in pool_data.pools.iter().take(top_n) {
        if let Some(changes) = pool_monitor.get_changes(&pool_info.id, 5).await {
            result.push_str(&format!(
                "🔄 {} ({}/{})\n\
                 💰 ${:.6}\n\
                 📈 Price: 5m:{:.2}% | 15m:{:.2}% | 1h:{:.2}% | 24h:{:.2}%\n\
                 📊 Vol: ${:.2}M\n\
                 📊 Vol Chg: 5m:{:.2}% | 15m:{:.2}% | 1h:{:.2}% | 24h:{:.2}%\n",
                pool_info.id,
                pool_info.symbol_a,
                pool_info.symbol_b,
                pool_info.price,
                changes.price_change_5m,
                changes.price_change_15m,
                changes.price_change_1h,
                changes.price_change_24h,
                pool_info.volume_24h / 1_000_000.0,
                changes.volume_change_5m,
                changes.volume_change_15m,
                changes.volume_change_1h,
                changes.volume_change_24h,
            ));

            // 警报检查
            if changes.price_change_5m.abs() > price_alert {
                result.push_str(&format!(
                    "⚠️ 价格5分钟变化显著: {:.2}%\n",
                    changes.price_change_5m
                ));
            }
            if changes.volume_change_5m.abs() > volume_alert {
                result.push_str(&format!(
                    "⚠️ 交易量5分钟变化显著: {:.2}%\n",
                    changes.volume_change_5m
                ));
            }

            result.push_str("----------------------\n");
        }
    }

    result
}
