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

use std::pin::pin;

/// A stream of bytes produced asynchronously.
pub trait SinglePassSource {
    type Error;

    fn next(&mut self) -> impl Future<Output = Option<Result<bytes::Bytes, Self::Error>>>;

    /// Returns the remaining bytes hint.
    /// 
    /// The client library uses this hint to select an optimal RPC for the given
    /// object size.
    fn size_hint(&self) -> (u64, Option<u64>);
}

/// A stream of bytes produced asynchronously with support for rewind.
/// 
/// The client library consumes implementations of this trait in the upload
/// functions. This trait can be used with files, in-memory buffers, 
pub trait MultipassSource {
    type Error;

    fn seek(&mut self, offset: u64) -> impl Future<Output = std::result::Result<(), Self::Error>>;
    fn next(&mut self) -> impl Future<Output = Option<Result<bytes::Bytes, Self::Error>>> + Send;

    /// Returns the remaining bytes hint.
    /// 
    /// The client library uses this hint to select an optimal RPC for the given
    /// object size.
    fn size_hint(&self) -> (u64, Option<u64>);
}

impl<T> MultipassSource for T
where T: tokio::io::AsyncSeek + tokio::io::AsyncRead + std::marker::Unpin + Send {
    type Error = std::io::Error;

    async fn seek(&mut self, offset: u64) -> std::result::Result<(), Self::Error> {
        use tokio::io::AsyncSeekExt;
        AsyncSeekExt::seek(self, std::io::SeekFrom::Start(offset)).await?;
        Ok(())
    }
    async fn next(&mut self) -> Option<Result<bytes::Bytes, Self::Error>> {
        use tokio::io::AsyncReadExt;
        let mut buf = Vec::with_capacity(1024 * 1024);
        let mut a = pin!(self);
        match a.read(&mut buf).await {
            Err(e) => Some(Err(e)),
            Ok(n) if n == 0 => None,
            Ok(_) => Some(Ok(bytes::Bytes::from_owner(buf))),
        }
    }
    fn size_hint(&self) -> (u64, Option<u64>) {
        (0, None)
    }
}



impl<T> SinglePassSource for T where T: MultipassSource {
    type Error = T::Error;

    async fn next(&mut self) -> Option<Result<bytes::Bytes, Self::Error>> {
        T::next(self).await
    }
    fn size_hint(&self) -> (u64, Option<u64>) {
        T::size_hint(&self)
    }
}
