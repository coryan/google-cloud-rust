# Error Handling

Applications often need to branch based on the error returned by the client library. Retryable errors are handled automatically by the library's policy-based retry loop.

## Extracting Service Errors

When an RPC fails for a service-related reason (e.g., resource not found), you can match on the `google_cloud_gax::error::Error` to check the `status` code.

```rust
use google_cloud_gax::error::Error;
use google_cloud_rpc::model::Code;

let response = client.update_secret(/* ... */).send().await;

match response {
    Ok(result) => {
        println!("Success!");
    }
    Err(Error::Rpc(status)) if status.code == Code::NotFound as i32 => {
        // Handle the "Not Found" case, e.g., by creating the resource first.
        println!("Secret not found, creating it...");
        // client.create_secret(...).send().await?;
    }
    Err(e) => {
        // Handle other service or transient errors
        eprintln!("Operation failed: {:?}", e);
    }
}
```