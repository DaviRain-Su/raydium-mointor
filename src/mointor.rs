use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

// 定义监控项结构
struct MonitorItem {
    name: String,
    check_interval: Duration,
    check_fn: fn() -> anyhow::Result<String>,
}

// 定义监控服务
pub struct MonitorService {
    items: Vec<MonitorItem>,
}

impl MonitorService {
    pub fn new() -> Self {
        MonitorService { items: Vec::new() }
    }

    pub fn add_item(
        &mut self,
        name: &str,
        interval: Duration,
        check_fn: fn() -> anyhow::Result<String>,
    ) {
        self.items.push(MonitorItem {
            name: name.to_string(),
            check_interval: interval,
            check_fn,
        });
    }

    pub fn run(&self) {
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
