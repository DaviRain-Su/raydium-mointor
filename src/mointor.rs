use anyhow::Result;
use futures::future::join_all;
use log::{error, info, warn, LevelFilter};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::{
    sync::{broadcast, mpsc, Mutex},
    time::{self, Duration, Instant},
};

#[derive(Debug, Clone)]
pub enum MonitorStatus {
    OK(String),
    Warning(String),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct MonitorEvent {
    pub item_name: String,
    pub status: MonitorStatus,
    timestamp: Instant,
}

#[derive(Clone)]
pub struct MonitorItem {
    name: String,
    check_interval: Duration,
    // 修改函数类型为返回 Future 的函数
    check_fn: Arc<dyn Fn() -> Pin<Box<dyn Future<Output = Result<String>> + Send>> + Send + Sync>,
}

pub struct MonitorMetrics {
    last_check_time: Instant,
    last_status: MonitorStatus,
    check_count: u64,
    error_count: u64,
}

pub struct MonitorService {
    items: Arc<Mutex<Vec<MonitorItem>>>,
    metrics: Arc<Mutex<HashMap<String, MonitorMetrics>>>,
    pub tx: broadcast::Sender<MonitorEvent>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl MonitorService {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        MonitorService {
            items: Arc::new(Mutex::new(Vec::new())),
            metrics: Arc::new(Mutex::new(HashMap::new())),
            tx,
            shutdown_tx: None,
        }
    }

    // 修改 add_item 方法以正确处理异步函数
    pub async fn add_item<F, Fut>(&self, name: &str, interval: Duration, check_fn: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<String>> + Send + 'static,
    {
        let mut items = self.items.lock().await;
        items.push(MonitorItem {
            name: name.to_string(),
            check_interval: interval,
            check_fn: Arc::new(move || Box::pin(check_fn())),
        });
    }

    pub async fn run(&mut self) -> Result<()> {
        let (shutdown_tx, mut _shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        let items = self.items.clone();
        let tx = self.tx.clone();
        let metrics = self.metrics.clone();

        tokio::spawn(async move {
            let mut handles = vec![];
            let items = items.lock().await;

            for item in items.iter() {
                let item = item.clone();
                let tx = tx.clone();
                let metrics = metrics.clone();

                let handle = tokio::spawn(async move {
                    let mut interval = time::interval(item.check_interval);
                    loop {
                        interval.tick().await;
                        let start = Instant::now();

                        // 执行异步检查函数
                        let check_future = (item.check_fn)();
                        let result = check_future.await;

                        let status = match result {
                            Ok(message) => MonitorStatus::OK(message),
                            Err(e) => MonitorStatus::Error(e.to_string()),
                        };

                        let mut metrics = metrics.lock().await;
                        let metric = metrics.entry(item.name.clone()).or_insert(MonitorMetrics {
                            last_check_time: start,
                            last_status: status.clone(),
                            check_count: 0,
                            error_count: 0,
                        });

                        metric.last_check_time = start;
                        metric.last_status = status.clone();
                        metric.check_count += 1;

                        if matches!(status, MonitorStatus::Error(_)) {
                            metric.error_count += 1;
                        }

                        if tx
                            .send(MonitorEvent {
                                item_name: item.name.clone(),
                                status,
                                timestamp: start,
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                });

                handles.push(handle);
            }

            join_all(handles).await;
        });

        Ok(())
    }

    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
        info!("Monitor service stopped");
    }
}
