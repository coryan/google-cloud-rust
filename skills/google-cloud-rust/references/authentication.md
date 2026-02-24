# Overriding Default Authentication

By default, the Google Cloud client libraries use Application Default Credentials (ADC). However, you can configure alternative authentication methods using the `google-cloud-auth` crate.

## Dependencies

Ensure your project depends on `google-cloud-auth`:

```toml
cargo add google-cloud-auth
```

## 1. Using API Keys

API keys are simple text strings that grant access to specific Google Cloud services. They are typically used for development or simpler public applications.

```rust
use google_cloud_auth::credentials::api_key_credentials::Builder as ApiKeyCredentialsBuilder;
// Example using a hypothetical generic Client
// use google_cloud_storage::client::Storage as Client;

pub async fn client_with_api_key(api_key: &str) -> anyhow::Result<Client> {
    // 1. Create API key credentials
    let credentials = ApiKeyCredentialsBuilder::new(api_key).build();
    
    // 2. Initialize the client with the custom credentials
    let client = Client::builder()
        .with_credentials(credentials)
        .build()
        .await?;
        
    Ok(client)
}
```

## 2. Using Service Account Impersonation

Service Account Impersonation allows you to make API calls on behalf of a service account without downloading its keys, providing short-lived credentials for enhanced security.

```rust
use google_cloud_auth::credentials::Builder as AdcCredentialsBuilder;
use google_cloud_auth::credentials::impersonated::Builder as ImpersonatedCredentialsBuilder;
// Example using a hypothetical generic Client
// use google_cloud_storage::client::Storage as Client;

/// `target_principal`: The email or unique ID of the target service account.
/// Example: `my-service-account@my-project.iam.gserviceaccount.com`
pub async fn client_with_impersonation(target_principal: &str) -> anyhow::Result<Client> {
    // 1. Create base Application Default Credentials
    let base_credentials = AdcCredentialsBuilder::default().build()?;

    // 2. Create impersonated credentials from the base credentials
    let credentials = ImpersonatedCredentialsBuilder::from_source_credentials(base_credentials)
        .with_target_principal(target_principal)
        .build()?;
        
    // 3. Initialize the client with the custom credentials
    let client = Client::builder()
        .with_credentials(credentials)
        .build()
        .await?;
        
    Ok(client)
}
```