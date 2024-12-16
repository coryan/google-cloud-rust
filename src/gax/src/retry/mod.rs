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

pub trait Policy: Send {
    fn may_retry(&self) -> bool {
        return true;
    }
    fn on_failure(&mut self, error: crate::error::Error) -> crate::Result<()>;
}

pub trait PolicyProvider: Send + Sync {
    fn start(&self) -> Box<dyn Policy>;
}

pub fn new_provider<T>(prototype: T) -> Arc<dyn PolicyProvider + Send + Sync + 'static>
where
    T: Policy + Clone + Send + Sync + 'static,
{
    Arc::new(GenericPolicyProvider::new(prototype))
}

struct GenericPolicyProvider<T: Policy + Clone + Send + Sync + 'static> {
    prototype: T,
}

impl<T> GenericPolicyProvider<T>
where
    T: Policy + Clone + Send + Sync + 'static,
{
    pub fn new(prototype: T) -> Self {
        Self { prototype }
    }
}

impl<T> PolicyProvider for GenericPolicyProvider<T>
where
    T: Policy + Clone + Send + Sync + 'static,
{
    fn start(&self) -> Box<dyn Policy> {
        Box::new(self.prototype.clone())
    }
}

#[derive(Clone, Debug)]
pub struct NoRetry;

impl Policy for NoRetry {
    fn may_retry(&self) -> bool {
        return false;
    }
    fn on_failure(&mut self, error: crate::error::Error) -> crate::Result<()> {
        return Err(error);
    }
}

#[derive(Clone, Debug)]
pub struct LimitedErrorCount {
    count: u32,
    maximum: u32,
}

impl LimitedErrorCount {
    pub fn new(maximum: u32) -> Self {
        Self { count: 0, maximum }
    }
}

impl Policy for LimitedErrorCount {
    fn may_retry(&self) -> bool {
        return true;
    }
    fn on_failure(&mut self, error: crate::error::Error) -> crate::Result<()> {
        if self.count < self.maximum {
            self.count += 1;
            Ok(())
        } else {
            // Should we chain these? Maybe "retry policy exhaused on {e}"?
            Err(error)
        }
    }
}
