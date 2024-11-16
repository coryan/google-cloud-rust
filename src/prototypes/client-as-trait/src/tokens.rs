// Copyright 2024 Google LLC
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

use std::sync::Arc;
use std::sync::Mutex;

type Error = Box<dyn std::error::Error>;

#[derive(Clone, Debug)]
pub struct AccessToken {
    pub value: String,
    /// The token expiration time. 
    pub expiration: std::time::Instant,
    /// Do not refresh before this time. Some sources cache the access token,
    /// refreshing too early just causes load.
    pub refresh_after: Option<std::time::Instant>,
}

#[async_trait::async_trait]
pub trait AccessTokenSource {
    async fn token(&self) -> Result<AccessToken, Error>;
}

#[derive(Clone)]
pub struct CachedAccessToken {
    inner: Arc<Impl>,
}

struct Impl {
    source: Box<dyn AccessTokenSource + Send + Sync>,
    current: Mutex<Option<AccessToken>>,
}

impl CachedAccessToken {
    pub async fn from<T: AccessTokenSource + Send + Sync + 'static>(source: T) -> Self {
        let inner = Impl { 
            source: Box::from(source),
            current: Mutex::from(None)
        };
        let inner = Arc::new(inner);
        let _ = tokio::spawn(Self::refresh_loop(inner.clone()));
        Self { inner }
    }

    async fn refresh_loop(inner: Arc<Impl>) {
        loop {
            let refresh = Self::refresh_attempt(&inner).await;
            let refresh = refresh.unwrap_or(std::time::Instant::now() + std::time::Duration::from_secs(30));
            tokio::time::sleep_until(refresh.into()).await;
        }
    }

    async fn refresh_attempt(inner: &Arc<Impl>) -> Result<std::time::Instant, Error> {
        let token = inner.source.token().await?;
        let mut refresh = token.expiration - std::time::Duration::from_secs(30);
        refresh = token.refresh_after.unwrap_or(refresh);

        let mut current = inner.current.lock().map_err(|e| format!("cannot acquire lock in refresh loop {e:?}"))?;
        *current = Some(token);

        Ok(refresh)
    }

    fn sync_token(&self) -> Result<AccessToken, Error> {
        let current  = self.inner.current.lock().map_err(|e| format!("cannot acquire lock in refresh loop {e:?}"))?;
        let result = current.as_ref().ok_or( "token unavailable")?;
        Ok(result.clone())
    }
}

#[async_trait::async_trait]
impl AccessTokenSource for CachedAccessToken {
    async fn token(&self) -> Result<AccessToken, Error> {
        self.sync_token()
    }
}

pub async fn default_credentials() -> Result<Box<dyn AccessTokenSource + Send + Sync>, Error> {
    let config = auth::CredentialConfig::builder()
            .scopes(
                ["https://www.googleapis.com/auth/cloud-platform"]
                    .map(str::to_string)
                    .to_vec(),
            )
            .build()?;
    let credentials = auth::Credential::find_default(config).await?;
    let adapter  = CredentialsAdapter::new(credentials);
//    let cached = CachedAccessToken::from(adapter).await;
    Ok(Box::new(adapter))
}

struct CredentialsAdapter {
    inner: auth::Credential,
}

impl CredentialsAdapter {
    pub fn new(inner: auth::Credential) -> Self {
        Self{inner}
    }
}

#[async_trait::async_trait]
impl AccessTokenSource for CredentialsAdapter {
    async fn token(&self) -> Result<AccessToken, Error> {
        let token = self.inner.access_token().await?;
        let hack = chrono::Utc::now();
        let hack = token.expires().map(|v| v - hack).map(|v| v.to_std()).transpose()?;
        let hack = hack.map(|v| std::time::Instant::now() + v);
        let refresh = hack.map(|v| v - std::time::Duration::from_secs(60));
        let hack = hack.unwrap_or_else(|| std::time::Instant::now() + std::time::Duration::from_secs(24 * 60 * 60));
        Ok(AccessToken {
            value: token.value,
            expiration: hack,
            refresh_after: refresh,
        })
    }
}
