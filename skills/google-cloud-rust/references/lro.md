# Long-Running Operations (LROs)

Many APIs expose methods that take significant time. The client library provides helpers to work with LROs.

Dependencies needed: `tokio` (with `full,macros` features) and `google-cloud-lro`.

## Automatic Polling

The simplest way to handle an LRO is to use the `Poller` trait and `.until_done()`. Instead of `.send().await`, you use `.poller().until_done().await`.

```rust
use google_cloud_lro::Poller;
// Example: Storage rename_folder
let operation_result = client
    .rename_folder()
    .set_bucket("my-bucket".to_string())
    .set_source_folder("old-folder".to_string())
    .set_destination_folder("new-folder".to_string())
    .poller()
    .until_done()
    .await?;
```

## Polling with Intermediate Results

If you need partial progress metadata:

```rust
use google_cloud_lro::Poller;
use std::time::Duration;
use tokio::time::sleep;

let mut poller = client
    .rename_folder()
    .set_bucket("my-bucket".to_string())
    .set_source_folder("old-folder".to_string())
    .set_destination_folder("new-folder".to_string())
    .poller();

loop {
    if let Some(result) = poller.poll().await? {
        println!("Operation completed!");
        break;
    }
    
    // You can access metadata here if the service provides it
    println!("Operation still running...");
    sleep(Duration::from_millis(500)).await;
}
```