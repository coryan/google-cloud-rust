# Client Setup & Configuration

## Initializing a Client

Clients are the main abstraction to interface with specific Google Cloud services. They are implemented as Rust structs with methods corresponding to each RPC.

1. **Dependency:** Add the crate to `Cargo.toml` (e.g., `cargo add google-cloud-secretmanager-v1`).
2. **Initialization:** Call `Client::builder()` to obtain a `ClientBuilder` and then call `build()`.

```rust
// Example: Secret Manager
use google_cloud_secretmanager_v1::client::Client;

async fn initialize_client() -> anyhow::Result<Client> {
    let client = Client::builder().build().await?;
    Ok(client)
}
```

## Making an RPC

Each request is represented by a method that returns a request builder.

```rust
// Example: Listing locations
let response = client
    .list_locations()
    .set_name("projects/123456789012".to_string())
    .send()
    .await?;

for location in response.locations {
    println!("{}", location.name);
}
```