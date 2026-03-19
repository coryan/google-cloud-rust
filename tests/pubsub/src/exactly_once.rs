// Copyright 2026 Google LLC
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

use google_cloud_pubsub::client::{Publisher, Subscriber};
use google_cloud_pubsub::model::Message;
use google_cloud_pubsub::subscriber::handler::Handler;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::task::JoinSet;
use tokio::time::Instant;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;

const MESSAGE_COUNT: usize = 1_000_000;
const BATCH_SIZE: usize = 1_000;
const RECV_BATCH_SIZE: usize = 10_000;
const WORKER_COUNT: usize = 32;
static MESSAGES_RECV: AtomicUsize = AtomicUsize::new(0);

pub async fn roundtrip(topic_name: &str, subscription_name: &str) -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_level(true)
        .with_thread_ids(true)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    tracing::info!("testing exactly once subscription");
    let publisher = Publisher::builder(topic_name).build().await?;

    let workers =
        JoinSet::from_iter((0..WORKER_COUNT).map(|i| worker(i, subscription_name.to_string())));

    let mut publish = JoinSet::new();
    for i in 0..MESSAGE_COUNT {
        publish.spawn(publisher.publish(Message::new().set_data(format!("{i}"))));
        if publish.len() >= BATCH_SIZE {
            let drain = std::mem::replace(&mut publish, JoinSet::new());
            drain
                .join_all()
                .await
                .into_iter()
                .try_fold((), |_, r| r.map(|_| ()))?;
        }
        if i > 0 && i % 100_000 == 0 {
            tracing::info!("Successfully published {i} messages");
        }
    }
    publish
        .join_all()
        .await
        .into_iter()
        .try_fold((), |_, r| r.map(|_| ()))?;

    tracing::info!("Waiting for subscribe tasks");
    workers
        .join_all()
        .await
        .into_iter()
        .try_fold((), |_, r| r)
        .inspect_err(|e| tracing::error!("error in worker task: {e:?}"))?;
    tracing::info!("successfully confirmed all messages");

    Ok(())
}

pub async fn nack_then_confirmed_ack(
    topic_name: &str,
    subscription_name: &str,
) -> anyhow::Result<()> {
    tracing::info!("testing exactly once nack then confirmed ack");
    let publisher = Publisher::builder(topic_name).build().await?;
    let subscriber = Subscriber::builder().build().await?;
    let mut stream = subscriber.subscribe(subscription_name).build();

    publisher
        .publish(Message::new().set_data("Hello, World!"))
        .await?;
    tracing::info!("successfully published message");

    let Some((m, Handler::ExactlyOnce(h))) = stream.next().await.transpose()? else {
        unreachable!("exactly-once delivery is enabled, and the stream stays open.")
    };
    assert_eq!(m.data, "Hello, World!");
    // Nack the message by dropping.
    drop(h);

    let Some((m, Handler::ExactlyOnce(h))) = stream.next().await.transpose()? else {
        unreachable!("exactly-once delivery is enabled, and the stream stays open.")
    };
    assert_eq!(m.data, "Hello, World!");
    h.confirmed_ack()
        .await
        .inspect_err(|e| tracing::info!("received unexpected confirm_ack error: {e:?}"))?;

    tracing::info!("successfully confirmed the message");
    Ok(())
}

async fn worker(id: usize, subscription_name: String) -> anyhow::Result<()> {
    let client = Subscriber::builder().build().await?;
    let mut acks = JoinSet::new();
    let mut stream = client
        .subscribe(subscription_name)
        .set_max_outstanding_messages(3 * RECV_BATCH_SIZE as i64)
        .set_max_outstanding_bytes(16 * 1024 * RECV_BATCH_SIZE as i64)
        .build();
    let mut start = Instant::now();
    tracing::info!("Worker[{id}] running");
    while let Some((_, Handler::ExactlyOnce(h))) = stream.next().await.transpose()? {
        acks.spawn(h.confirmed_ack());
        let recv = MESSAGES_RECV.fetch_add(1, Ordering::AcqRel);
        if recv + id * RECV_BATCH_SIZE >= MESSAGE_COUNT {
            break;
        }
        if acks.len() >= RECV_BATCH_SIZE {
            let count = acks.len();
            tracing::info!(
                "Worker[{id}] collected {count} / {} acks in {:?}",
                MESSAGE_COUNT - recv,
                start.elapsed()
            );
            start = Instant::now();
            let drain = std::mem::replace(&mut acks, JoinSet::new());
            drain.join_all().await.into_iter().try_fold((), |_, r| r)?;
            tracing::info!("Worker[{id}] drained {count} acks in {:?}", start.elapsed());
        }
    }
    let count = acks.len();
    acks.join_all().await.into_iter().try_fold((), |_, r| r)?;
    tracing::info!(
        "Worker[{id}] drained last {count} acks in {:?}",
        start.elapsed()
    );
    Ok(())
}
