# Error Handling

Applications often need to branch based on the error returned by the client library. Retryable errors are handled automatically by the library's policy-based retry loop.

## Extracting Service Errors

When an RPC fails for a service-related reason (e.g., resource not found), you can inspect the error by calling its `.status()` method to check the RPC `Code`.

```rust
use google_cloud_secretmanager_v1::client::SecretManagerService;
use google_cloud_gax::error::Error;
use google_cloud_gax::error::rpc::Code;

pub async fn handle_error(client: &SecretManagerService) -> anyhow::Result<()> {
    let response = client
        .add_secret_version()
        // ... set request parameters
        .send()
        .await;

    match response {
        Ok(_result) => {
            println!("Success!");
            Ok(())
        }
        Err(e) => {
            // Check if the error has an associated RPC status
            if let Some(status) = e.status() {
                // Compare the status code
                if status.code == Code::NotFound {
                    println!("Secret not found, creating it...");
                    // Handle the "Not Found" case, e.g., by creating the resource first.
                    return Ok(());
                }
            }
            
            // Handle other service or transient errors
            eprintln!("Operation failed: {:?}", e);
            Err(e.into())
        }
    }
}
```