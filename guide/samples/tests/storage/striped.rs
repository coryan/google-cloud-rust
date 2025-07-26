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

use google_cloud_storage::client::Storage;
use google_cloud_storage::client::StorageControl;

async fn seed(bucket_name: &str) -> anyhow::Result<()> {
    use google_cloud_storage::model::Object;
    use google_cloud_storage::model::compose_object_request::SourceObject;

    let buffer = String::from_iter(('a'..='z').cycle().take(1024 * 1024));

    let client = Storage::builder().build().await?;
    let seed = client
        .upload_object(bucket_name, "1MiB.txt", bytes::Bytes::from_owner(buffer))
        .send_unbuffered()
        .await?;
    println!(
        "Uploaded object {}, size={}KiB",
        seed.name,
        seed.size / 1024
    );

    let control = StorageControl::builder().build().await?;
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

    Ok(())
}

pub async fn test(bucket_name: &str) -> anyhow::Result<()> {
    seed(bucket_name).await?;
    Ok(())
}
