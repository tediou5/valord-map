# Valord Map

## Overview

A dictionary sorted by values.

You can directly sort based on your data structure or implement OrdBy to specify sorting based on a particular field.

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]

[crates-badge]: https://img.shields.io/crates/v/valord_map.svg
[crates-url]: https://crates.io/crates/valord-map
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/tediou5/valord-map/blob/master/LICENSE

## Example

Watch a key in etcd, insert into ValordMap, and trigger a notification when the maximum value changes.

```toml
[dependencies]
valord-map = { version = "*", features = ["watcher"] }

anyhow = "1.0.83"
etcd-client = { version = "0.12.4", features = ["tls"] }
tokio = { version = "1.37.0", features = ["full"] }
```

```no_run
use etcd_client::{Client, EventType, WatchOptions};
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;

use valord_map::{OrdBy, ValordMap};

#[derive(Debug)]
struct ValueWithInstant {
    value: u64,
    timestamp: Duration,
}

impl OrdBy for ValueWithInstant {
    type Target = u64;

    fn ord_by<'a>(&'a self) -> &Self::Target {
        &self.value
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    let nodes = ValordMap::new();
    let mut watcher = nodes.watcher();

    let client = Client::connect(["127.0.0.1:2379"], None).await.unwrap();
    let client: Arc<Mutex<Client>> = Mutex::new(client).into();

    let client_c = client.clone();
    tokio::spawn(async move { watch(client_c, nodes).await.unwrap() });

    let client_c = client.clone();
    tokio::spawn(async move {
        println!("node1 start put");
        put(client_c, "node1", 0).await
    });
    let client_c = client.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(3)).await;
        println!("slow node start put");
        put(client_c, "node_slow", 0).await
    });
    let client_c = client.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(1)).await;
        println!("node2 start put");
        put(client_c, "node2", 1).await
    });
    let client_c = client.clone();
    tokio::spawn(async move {
        println!("node3 start put");
        put(client_c, "node3", 0).await
    });

    println!("watching changed...");
    loop {
        let header = watcher.head_changed().await.unwrap().unwrap();
        println!(
            "watch header changed: {:?} at {:?}",
            header.value,
            header.timestamp.as_secs()
        );
    }
}

async fn put(client: Arc<Mutex<Client>>, node_id: &str, mut index: u64) -> anyhow::Result<()> {
    println!("create putter");
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        println!("put {node_id:?}: {index}");
        client
            .lock()
            .await
            .put(node_id, index.to_be_bytes(), None)
            .await?;
        index += 2;
    }
}

async fn watch(
    client: Arc<Mutex<Client>>,
    mut nodes: ValordMap<u64, String, ValueWithInstant>,
) -> anyhow::Result<()> {
    let option = WatchOptions::new().with_prefix();
    let (mut watcher, mut stream) = client.lock().await.watch("node", Some(option)).await?;
    println!("create watcher {}", watcher.watch_id());

    while let Some(resp) = stream.message().await? {
        for event in resp.events() {
            match event.event_type() {
                EventType::Put => {
                    if let Some(kv) = event.kv() {
                        let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                        let key = kv.key_str()?.to_string();
                        let value = kv.value();
                        let mut value_bytes = [0u8; 8];
                        value_bytes.copy_from_slice(value);
                        let value = u64::from_be_bytes(value_bytes);

                        let value_order_by = ValueWithInstant {
                            value,
                            timestamp: time,
                        };

                        nodes.insert(key, value_order_by)
                    }
                }
                EventType::Delete => watcher.cancel_by_id(resp.watch_id()).await?,
            }
        }
    }

    Ok(())
}

```
