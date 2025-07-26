// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// ANCHOR: all

// ANCHOR: seed-function
async fn seed(bucket_name: &str) -> anyhow::Result<()> {
    // ANCHOR_END: seed-function
    // ANCHOR: seed-client
    use google_cloud_storage::client::Storage;
    use google_cloud_storage::client::StorageControl;
    use google_cloud_storage::model::Object;
    use google_cloud_storage::model::compose_object_request::SourceObject;

    let client = Storage::builder().build().await?;
    let control = StorageControl::builder().build().await?;
    // ANCHOR_END: seed-client

    // ANCHOR: upload-1MiB
    let buffer = String::from_iter(('a'..='z').cycle().take(1024 * 1024));
    let seed = client
        .upload_object(bucket_name, "1MiB.txt", bytes::Bytes::from_owner(buffer))
        .send_unbuffered()
        .await?;
    println!(
        "Uploaded object {}, size={}KiB",
        seed.name,
        seed.size / 1024
    );
    // ANCHOR_END: upload-1MiB

    // ANCHOR: compose-32
    let seed_32 = control
        .compose_object()
        .set_destination(Object::new().set_bucket(bucket_name).set_name("32MiB.txt"))
        .set_source_objects((0..32).map(|_| {
            SourceObject::new()
                .set_name(&seed.name)
                .set_generation(seed.generation)
        }))
        .send()
        .await?;
    println!(
        "Created object {}, size={}MiB",
        seed.name,
        seed.size / (1024 * 1024)
    );
    // ANCHOR_END: compose-32

    // ANCHOR: compose-1024
    let seed_1024 = control
        .compose_object()
        .set_destination(Object::new().set_bucket(bucket_name).set_name("1GiB.txt"))
        .set_source_objects((0..32).map(|_| {
            SourceObject::new()
                .set_name(&seed_32.name)
                .set_generation(seed_32.generation)
        }))
        .send()
        .await?;
    println!(
        "Created object {}, size={}MiB",
        seed.name,
        seed.size / (1024 * 1024)
    );
    // ANCHOR_END: compose-1024

    // ANCHOR: compose-GiB
    for s in [2, 4, 8, 16] {
        let name = format!("{s}GiB.txt");
        let target = control
            .compose_object()
            .set_destination(Object::new().set_bucket(bucket_name).set_name(&name))
            .set_source_objects((0..s).map(|_| {
                SourceObject::new()
                    .set_name(&seed_1024.name)
                    .set_generation(seed_1024.generation)
            }))
            .send()
            .await?;
        println!(
            "Created object {} size={}",
            target.name,
            target.size / (1024 * 1024)
        );
    }
    // ANCHOR_END: compose-GiB

    // ANCHOR: seed-function-end
    Ok(())
}
// ANCHOR_END: seed-function-end

// ANCHOR: download-function
use google_cloud_storage::client::Storage;
use google_cloud_storage::model::Object;
use tokio::io::AsyncWriteExt;

async fn download(
    bucket_name: &str,
    object_name: &str,
    stripe_size: usize,
    destination: &str,
) -> anyhow::Result<()> {
    // ANCHOR_END: download-function
    // ANCHOR: download-client
    use google_cloud_storage::client::StorageControl;

    let client = Storage::builder().build().await?;
    let control = StorageControl::builder().build().await?;
    // ANCHOR_END: download-client

    let metadata = control
        .get_object()
        .set_bucket(bucket_name)
        .set_object(object_name)
        .send()
        .await?;

    let file = tokio::fs::File::create(destination).await?;
    let limit = stripe_size as i64;
    let count = metadata.size / limit;

    let start = std::time::Instant::now();
    let stripes = (0..count)
        .map(|i| write_stripe(&client, &file, i * limit, limit, &metadata))
        .collect::<Vec<_>>();
    futures::future::join_all(stripes)
        .await
        .into_iter()
        .collect::<anyhow::Result<Vec<_>>>()?;

    let elapsed = std::time::Instant::now() - start;
    let bw = metadata.size as f64 / (1024.0 * 1024.0) / elapsed.as_secs_f64();
    println!("Completed download in {elapsed:?}, effective bandwidth = {bw:.2} MiB/s");

    // ANCHOR: download-function-end
    Ok(())
}
// ANCHOR_END: download-function-end

async fn write_stripe(
    client: &Storage,
    file: &tokio::fs::File,
    offset: i64,
    limit: i64,
    metadata: &Object,
) -> anyhow::Result<()> {
    use tokio::io::AsyncSeekExt;
    let mut writer = file.try_clone().await?;
    writer.seek(std::io::SeekFrom::Start(offset as u64)).await?;
    let mut reader = client
        .read_object(&metadata.bucket, &metadata.name)
        .with_generation(metadata.generation)
        .with_read_offset(offset)
        .with_read_limit(limit)
        .send()
        .await?;
    while let Some(b) = reader.next().await.transpose()? {
        writer.write(&b).await?;
    }
    Ok(())
}

// ANCHOR_END: all

pub async fn test(bucket_name: &str, destination: &str) -> anyhow::Result<()> {
    const MB: usize = 1024 * 1024;
    seed(bucket_name).await?;
    download(bucket_name, "32MiB.txt", 32 * MB, destination).await?;
    download(bucket_name, "32MiB.txt", MB, destination).await?;
    // download(bucket_name, "1GiB.txt", 1024 * MB, destination).await?;
    // download(bucket_name, "1GiB.txt", 512 * MB, destination).await?;
    // download(bucket_name, "4GiB.txt", 512 * MB, destination).await?;
    // download(bucket_name, "8GiB.txt", 512 * MB, destination).await?;
    // download(bucket_name, "16GiB.txt", 512 * MB, destination).await?;
    Ok(())
}
