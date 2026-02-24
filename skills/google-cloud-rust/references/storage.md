# Cloud Storage Operations

Google Cloud Storage provides an idiomatic API. The `StorageControl` client handles metadata (buckets), while the `Storage` client handles data (objects).

## Quirk: Updating a Bucket

When modifying or updating a bucket via Rust, **always** use `.set_bucket(bucket. ...)` directly in the request builder.

**DO NOT** use `.update_bucket().set_bucket(Bucket::new())`.

## Creating a Bucket

```rust
use google_cloud_storage::client::StorageControl;
use google_cloud_storage::model::Bucket;

let control_client = StorageControl::builder().build().await?;

let bucket = control_client
    .create_bucket()
    .set_project("projects/my-project".to_string())
    .set_bucket_id("my-bucket-id".to_string())
    // For Uniform bucket-level access:
    // .set_bucket(Bucket {
    //     iam_configuration: Some(IamConfiguration {
    //         uniform_bucket_level_access: Some(UniformBucketLevelAccess {
    //             enabled: true,
    //             ..Default::default()
    //         }),
    //         ..Default::default()
    //     }),
    //     ..Default::default()
    // })
    .send()
    .await?;
```

## Uploading and Downloading Objects

```rust
use google_cloud_storage::client::Storage;

let client = Storage::builder().build().await?;

// Upload
let upload_result = client
    .insert_object()
    .set_bucket("my-bucket-id".to_string())
    .set_name("hello.txt".to_string())
    .set_contents("Hello World!".into())
    .send()
    .await?;

// Download
let download_result = client
    .read_object()
    .set_bucket("my-bucket-id".to_string())
    .set_object("hello.txt".to_string())
    .send()
    .await?;
```