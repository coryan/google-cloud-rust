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

use super::active_read::ActiveRead;
use super::connector::{Connection, Connector};
use super::{Client, TonicStreaming};
use crate::error::ReadError;
use crate::google::storage::v2::{BidiReadObjectRequest, BidiReadObjectResponse, ObjectRangeData};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::Receiver;

type ReadResult<T> = std::result::Result<T, ReadError>;

#[derive(Debug)]
pub struct Worker<S> {
    next_range_id: i64,
    ranges: Arc<Mutex<HashMap<i64, ActiveRead>>>,
    connection: Connection<S>,
}

impl<S> Worker<S> {
    pub fn new(connection: Connection<S>) -> Self {
        let ranges = Arc::new(Mutex::new(HashMap::new()));
        Self {
            next_range_id: 0_i64,
            ranges,
            connection,
        }
    }
}

impl<S> Worker<S>
where
    S: TonicStreaming,
{
    pub async fn run<C>(
        mut self,
        mut connector: Connector<C>,
        mut rx: Receiver<ActiveRead>,
    ) -> crate::Result<()>
    where
        C: Client<Stream = S> + Clone + 'static,
    {
        loop {
            tokio::select! {
                m = self.next_message(&mut connector) => {
                    let Some(message) = m else {
                        break;
                    };
                    if let Err(e) = self.handle_response(message).await {
                        // An error in the response. These are not recoverable.
                        let error = Arc::new(e);
                        self.close_readers(error.clone()).await;
                        return Err(crate::Error::io(error));
                    }
                },
                r = rx.recv() => {
                    let Some(range) = r else {
                        return Ok(());
                    };
                    self.insert_range(range).await;
                },
            }
        }
        Ok(())
    }

    async fn next_message<C>(
        &mut self,
        connector: &mut Connector<C>,
    ) -> Option<BidiReadObjectResponse>
    where
        C: Client<Stream = S> + Clone + 'static,
    {
        let message = self.connection.rx.next_message().await;
        let status = match message {
            Ok(m) => return m,
            Err(status) => status,
        };
        let ranges: Vec<_> = self
            .ranges
            .lock()
            .await
            .iter()
            .map(|(id, r)| r.as_proto(*id))
            .collect();
        match connector.reconnect(status, ranges).await {
            Err(e) => {
                self.close_readers(Arc::new(e)).await;
                None
            }
            Ok((m, connection)) => {
                self.connection = connection;
                Some(m)
            }
        }
    }

    async fn close_readers(&mut self, error: Arc<crate::Error>) {
        let mut guard = self.ranges.lock().await;
        let closing: Vec<_> = guard
            .iter_mut()
            .map(|(_, pending)| pending.interrupted(error.clone()))
            .collect();
        let _ = futures::future::join_all(closing).await;
    }

    async fn insert_range(&mut self, range: ActiveRead) {
        let id = self.next_range_id;
        self.next_range_id += 1;

        let request = range.as_proto(id);
        self.ranges.lock().await.insert(id, range);
        let request = BidiReadObjectRequest {
            read_ranges: vec![request],
            ..BidiReadObjectRequest::default()
        };
        // Any errors here are recovered by the main background loop.
        if let Err(e) = self.connection.tx.send(request).await {
            tracing::error!("error sending read range request: {e:?}");
        }
    }

    async fn handle_response(&mut self, message: BidiReadObjectResponse) -> crate::Result<()> {
        let ranges = self.ranges.clone();
        let pending = message
            .object_data_ranges
            .into_iter()
            .map(|r| Self::handle_range_data(ranges.clone(), r))
            .collect::<Vec<_>>();
        let _ = futures::future::join_all(pending)
            .await
            .into_iter()
            .collect::<ReadResult<Vec<_>>>()
            .map_err(crate::Error::io)?; // TODO: think about the error type
        Ok(())
    }

    async fn handle_range_data(
        ranges: Arc<Mutex<HashMap<i64, ActiveRead>>>,
        response: ObjectRangeData,
    ) -> ReadResult<()> {
        let range = response
            .read_range
            .ok_or(ReadError::MissingRangeInBidiResponse)?;
        if response.range_end {
            let mut pending = ranges
                .lock()
                .await
                .remove(&range.read_id)
                .ok_or(ReadError::UnknownRange(range.read_id))?;
            pending
                .handle_data(response.checksummed_data, range, true)
                .await
        } else {
            let mut guard = ranges.lock().await;
            let pending = guard
                .get_mut(&range.read_id)
                .ok_or(ReadError::UnknownRange(range.read_id))?;
            pending
                .handle_data(response.checksummed_data, range, false)
                .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::mocks::{MockStream, MockStreamSender, MockTestClient, SharedMockClient};
    use super::super::tests::test_options;
    use super::*;
    use crate::google::storage::v2::BidiReadObjectSpec;

    #[tokio::test]
    async fn run_immediately_closed() -> anyhow::Result<()> {
        let (request_tx, _request_rx) = tokio::sync::mpsc::channel(1);
        let (response_tx, response_rx) = mock_stream();
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        let connection = Connection::new(request_tx, response_rx);
        let worker = Worker::new(connection);

        // Closing the stream without an error should not attempt a reconnect.
        drop(response_tx);
        let mut mock = MockTestClient::new();
        mock.expect_start().never();

        let connector = mock_connector(mock);
        let result = worker.run(connector, rx).await;
        assert!(result.is_ok(), "{result:?}");
        Ok(())
    }

    fn mock_connector(mock: MockTestClient) -> Connector<SharedMockClient> {
        let client = SharedMockClient::new(mock);

        let spec = BidiReadObjectSpec {
            bucket: "projects/_/buckets/test-bucket".into(),
            object: "test-object".into(),
            ..BidiReadObjectSpec::default()
        };

        Connector::new(spec, test_options(), client.clone())
    }

    fn mock_stream() -> (MockStreamSender, MockStream) {
        tokio::sync::mpsc::channel(10)
    }
}
