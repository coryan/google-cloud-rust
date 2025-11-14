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
use tokio::sync::mpsc::{Receiver, Sender};

type ReadResult<T> = std::result::Result<T, ReadError>;
type LoopResult<T> = std::result::Result<T, Arc<crate::Error>>;

#[derive(Debug)]
pub struct Worker<C> {
    next_range_id: i64,
    ranges: Arc<Mutex<HashMap<i64, ActiveRead>>>,
    connector: Connector<C>,
}

impl<C> Worker<C> {
    pub fn new(connector: Connector<C>) -> Self {
        let ranges = Arc::new(Mutex::new(HashMap::new()));
        Self {
            next_range_id: 0_i64,
            ranges,
            connector,
        }
    }
}

impl<C> Worker<C>
where
    C: Client + Clone + 'static,
    <C as Client>::Stream: TonicStreaming,
{
    pub async fn run(
        mut self,
        connection: Connection<C::Stream>,
        mut requests: Receiver<ActiveRead>,
    ) -> LoopResult<()> {
        let (mut rx, mut tx) = (connection.rx, connection.tx);
        // Note how this loop only exits when the `requests` queue is
        // closed. A successfully closed stream and unrecoverable errors
        // return immediately.
        loop {
            tokio::select! {
                m = rx.next_message() => {
                    match self.handle_response(m).await {
                        None => return Ok(()),
                        Some(Err(e)) => return Err(e),
                        Some(Ok(None)) => {},
                        Some(Ok(Some(connection))) => {
                            (rx, tx) = (connection.rx, connection.tx);
                        }
                    };
                },
                r = requests.recv() => {
                    let Some(range) = r else {
                        break;
                    };
                    self.insert_range(tx.clone(), range).await;
                },
            }
        }
        // No need to continue reading. The `requests` queue closes
        // only when the ObjectDescriptor is gone and when all the
        // associated ReadResponseReaders are gone.
        Ok(())
    }

    async fn handle_response(
        &mut self,
        message: tonic::Result<Option<BidiReadObjectResponse>>,
    ) -> Option<LoopResult<Option<Connection<C::Stream>>>> {
        let response = match message.transpose()? {
            Ok(r) => r,
            Err(status) => return self.reconnect(status).await,
        };

        if let Err(e) = self.handle_ranges(response.object_data_ranges).await {
            // An error in the response. These are not recoverable.
            let error = Arc::new(e);
            self.close_readers(error.clone()).await;
            return Some(Err(error));
        }
        Some(Ok(None))
    }

    async fn handle_ranges(&self, data: Vec<ObjectRangeData>) -> crate::Result<()> {
        let ranges = self.ranges.clone();
        let pending = data
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

    async fn reconnect(
        &mut self,
        status: tonic::Status,
    ) -> Option<LoopResult<Option<Connection<C::Stream>>>> {
        let ranges: Vec<_> = self
            .ranges
            .lock()
            .await
            .iter()
            .map(|(id, r)| r.as_proto(*id))
            .collect();
        let (response, connection) = match self.connector.reconnect(status, ranges).await {
            Err(e) => {
                let error = Arc::new(e);
                self.close_readers(error.clone()).await;
                return Some(Err(error));
            }
            Ok((m, connection)) => (m, connection),
        };
        if let Err(e) = self.handle_ranges(response.object_data_ranges).await {
            // An error in the response. These are not recoverable.
            // TODO: refactor to handle_ranges().
            let error = Arc::new(e);
            self.close_readers(error.clone()).await;
            return Some(Err(error));
        }
        Some(Ok(Some(connection)))
    }

    async fn close_readers(&mut self, error: Arc<crate::Error>) {
        let mut guard = self.ranges.lock().await;
        let closing: Vec<_> = guard
            .iter_mut()
            .map(|(_, pending)| pending.interrupted(error.clone()))
            .collect();
        let _ = futures::future::join_all(closing).await;
    }

    async fn insert_range(&mut self, tx: Sender<BidiReadObjectRequest>, reader: ActiveRead) {
        let id = self.next_range_id;
        self.next_range_id += 1;

        let request = reader.as_proto(id);
        self.ranges.lock().await.insert(id, reader);
        let request = BidiReadObjectRequest {
            read_ranges: vec![request],
            ..BidiReadObjectRequest::default()
        };
        // Any errors here are recovered by the main background loop.
        if let Err(e) = tx.send(request).await {
            tracing::error!("error sending read range request: {e:?}");
        }
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
    use super::super::tests::{proto_range_id, test_options};
    use super::*;
    use crate::google::storage::v2::{BidiReadObjectSpec, ChecksummedData};
    use crate::model_ext::ReadRange;
    use std::error::Error as _;
    use test_case::test_case;

    #[tokio::test]
    async fn run_immediately_closed() -> anyhow::Result<()> {
        let (request_tx, _request_rx) = tokio::sync::mpsc::channel(1);
        let (response_tx, response_rx) = mock_stream();
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        let connection = Connection::new(request_tx, response_rx);

        // Closing the stream without an error should not attempt a reconnect.
        drop(response_tx);
        let mut mock = MockTestClient::new();
        mock.expect_start().never();

        let connector = mock_connector(mock);
        let worker = Worker::new(connector);
        let result = worker.run(connection, rx).await;
        assert!(result.is_ok(), "{result:?}");
        Ok(())
    }

    #[test_case(true)]
    #[test_case(false)]
    #[tokio::test]
    async fn run_bad_response(range_end: bool) -> anyhow::Result<()> {
        let (request_tx, _request_rx) = tokio::sync::mpsc::channel(1);
        let (response_tx, response_rx) = mock_stream();
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        let connection = Connection::new(request_tx, response_rx);

        // Simulate a response for an unexpected read id.
        let response = BidiReadObjectResponse {
            object_data_ranges: vec![ObjectRangeData {
                read_range: Some(proto_range_id(0, 100, -123)),
                range_end,
                ..ObjectRangeData::default()
            }],
            ..BidiReadObjectResponse::default()
        };
        response_tx.send(Ok(response)).await?;
        let mut mock = MockTestClient::new();
        mock.expect_start().never();

        let connector = mock_connector(mock);
        let worker = Worker::new(connector);
        let err = worker.run(connection, rx).await.unwrap_err();
        assert!(err.is_transport(), "{err:?}");
        let source = err.source().and_then(|e| e.downcast_ref::<ReadError>());
        assert!(
            matches!(source, Some(ReadError::UnknownRange(r)) if *r == -123),
            "{err:?}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn run_stop_on_closed_requests() -> anyhow::Result<()> {
        let (request_tx, _request_rx) = tokio::sync::mpsc::channel(1);
        let (_response_tx, response_rx) = mock_stream();
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let connection = Connection::new(request_tx, response_rx);

        let mut mock = MockTestClient::new();
        mock.expect_start().never();

        let connector = mock_connector(mock);
        let worker = Worker::new(connector);
        drop(tx);
        worker.run(connection, rx).await?;
        Ok(())
    }

    #[tokio::test]
    async fn run_partial_read() -> anyhow::Result<()> {
        let (request_tx, mut request_rx) = tokio::sync::mpsc::channel(1);
        let (response_tx, response_rx) = mock_stream();
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let connection = Connection::new(request_tx, response_rx);

        let mut mock = MockTestClient::new();
        mock.expect_start().never();
        let connector = mock_connector(mock);
        let worker = Worker::new(connector);
        let join = tokio::spawn(async move { worker.run(connection, rx).await });

        let mut reader = mock_reader(&tx, ReadRange::segment(100, 200)).await;
        let request = request_rx
            .recv()
            .await
            .expect("request queue is not closed");
        assert!(request.read_object_spec.is_none(), "{request:?}");
        assert_eq!(request.read_ranges.len(), 1, "{request:?}");
        let request = request.read_ranges.first().unwrap();
        assert_eq!(request.read_offset, 100);
        assert_eq!(request.read_length, 200);

        // Simulate a response for an unexpected read id.
        let content = bytes::Bytes::from_owner(String::from_iter((0..100).map(|_| 'x')));
        let response = BidiReadObjectResponse {
            object_data_ranges: vec![ObjectRangeData {
                read_range: Some(proto_range_id(100, content.len() as i64, request.read_id)),
                checksummed_data: Some(ChecksummedData {
                    content: content.clone(),
                    ..ChecksummedData::default()
                }),
                ..ObjectRangeData::default()
            }],
            ..BidiReadObjectResponse::default()
        };
        response_tx.send(Ok(response)).await?;

        let got = reader.recv().await;
        assert!(matches!(got, Some(Ok(ref b)) if *b == content), "{got:?}");

        drop(tx);
        join.await??;
        Ok(())
    }

    #[tokio::test]
    async fn run_full_read() -> anyhow::Result<()> {
        let (request_tx, mut request_rx) = tokio::sync::mpsc::channel(1);
        let (response_tx, response_rx) = mock_stream();
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let connection = Connection::new(request_tx, response_rx);

        let mut mock = MockTestClient::new();
        mock.expect_start().never();
        let connector = mock_connector(mock);
        let worker = Worker::new(connector);
        let join = tokio::spawn(async move { worker.run(connection, rx).await });

        let mut reader = mock_reader(&tx, ReadRange::segment(100, 100)).await;
        let request = request_rx
            .recv()
            .await
            .expect("request queue is not closed");
        assert!(request.read_object_spec.is_none(), "{request:?}");
        assert_eq!(request.read_ranges.len(), 1, "{request:?}");
        let request = request.read_ranges.first().unwrap();
        assert_eq!(request.read_offset, 100);
        assert_eq!(request.read_length, 100);

        // Simulate a response for an unexpected read id.
        let content = bytes::Bytes::from_owner(String::from_iter((0..100).map(|_| 'x')));
        let response = BidiReadObjectResponse {
            object_data_ranges: vec![ObjectRangeData {
                read_range: Some(proto_range_id(100, content.len() as i64, request.read_id)),
                range_end: true,
                checksummed_data: Some(ChecksummedData {
                    content: content.clone(),
                    ..ChecksummedData::default()
                }),
                ..ObjectRangeData::default()
            }],
            ..BidiReadObjectResponse::default()
        };
        response_tx.send(Ok(response)).await?;

        let got = reader.recv().await;
        assert!(matches!(got, Some(Ok(ref b)) if *b == content), "{got:?}");
        let got = reader.recv().await;
        assert!(got.is_none(), "{got:?}");

        drop(tx);
        join.await??;
        Ok(())
    }

    async fn mock_reader(
        requests: &Sender<ActiveRead>,
        range: ReadRange,
    ) -> Receiver<ReadResult<bytes::Bytes>> {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let reader = ActiveRead::new(tx, range.0);
        requests
            .send(reader)
            .await
            .expect("requests queue is not closed in tests");
        rx
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
