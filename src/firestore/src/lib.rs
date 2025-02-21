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

pub use gax::error::Error;
pub use gax::Result;
// TODO(#1426) - fix references to the `client::Firestore::*` methods
#[allow(rustdoc::broken_intra_doc_links)]
pub mod model;

pub(crate) mod google {
    pub mod firestore {
        pub mod v1 {
            include!("protos/firestore/google.firestore.v1.rs");
            //            include!("protos/firestore/convert.rs");
        }
    }
    pub mod rpc {
        include!("protos/firestore/google.rpc.rs");
    }
    pub mod r#type {
        include!("protos/firestore/google.r#type.rs");
    }
}

#[cfg(any())]
impl wkt::prost::Convert<rpc::model::Status> for google::rpc::Status {
    fn cnv(self) -> rpc::model::Status {
        rpc::model::Status::new()
            .set_code(self.code)
            .set_message(self.message)
            .set_details(self.details.into_iter().filter_map(from_prost_error_detail))
    }
}

#[cfg(any())]
impl wkt::prost::Convert<google::rpc::Status> for rpc::model::Status {
    fn cnv(self) -> google::rpc::Status {
        google::rpc::Status {
            code: self.code,
            message: self.message,
            details: self
                .details
                .into_iter()
                .filter_map(to_prost_error_detail)
                .collect(),
        }
    }
}

#[cfg(any())]
fn from_prost_error_detail(detail: prost_types::Any) -> Option<wkt::Any> {
    use wkt::prost::Convert;
    let any = match detail.type_url.as_str() {
        "type.googleapis.com/google.rpc.BadRequest" => detail
            .to_msg::<google::rpc::BadRequest>()
            .ok()
            .map(|v| v.cnv())
            .as_ref()
            .map(wkt::Any::try_from),
        "type.googleapis.com/google.rpc.DebugInfo" => detail
            .to_msg::<google::rpc::DebugInfo>()
            .ok()
            .map(|v| v.cnv())
            .as_ref()
            .map(wkt::Any::try_from),
        "type.googleapis.com/google.rpc.ErrorInfo" => detail
            .to_msg::<google::rpc::ErrorInfo>()
            .ok()
            .map(|v| v.cnv())
            .as_ref()
            .map(wkt::Any::try_from),
        "type.googleapis.com/google.rpc.Help" => detail
            .to_msg::<google::rpc::Help>()
            .ok()
            .map(|v| v.cnv())
            .as_ref()
            .map(wkt::Any::try_from),
        "type.googleapis.com/google.rpc.LocalizedMessage" => detail
            .to_msg::<google::rpc::LocalizedMessage>()
            .ok()
            .map(|v| v.cnv())
            .as_ref()
            .map(wkt::Any::try_from),
        "type.googleapis.com/google.rpc.PreconditionFailure" => detail
            .to_msg::<google::rpc::PreconditionFailure>()
            .ok()
            .map(|v| v.cnv())
            .as_ref()
            .map(wkt::Any::try_from),
        "type.googleapis.com/google.rpc.QuotaFailure" => detail
            .to_msg::<google::rpc::QuotaFailure>()
            .ok()
            .map(|v| v.cnv())
            .as_ref()
            .map(wkt::Any::try_from),
        "type.googleapis.com/google.rpc.RequestInfo" => detail
            .to_msg::<google::rpc::RequestInfo>()
            .ok()
            .map(|v| v.cnv())
            .as_ref()
            .map(wkt::Any::try_from),
        "type.googleapis.com/google.rpc.ResourceInfo" => detail
            .to_msg::<google::rpc::ResourceInfo>()
            .ok()
            .map(|v| v.cnv())
            .as_ref()
            .map(wkt::Any::try_from),
        "type.googleapis.com/google.rpc.RetryInfo" => detail
            .to_msg::<google::rpc::RetryInfo>()
            .ok()
            .map(|v| v.cnv())
            .as_ref()
            .map(wkt::Any::try_from),
        _ => None::<std::result::Result<wkt::Any, wkt::AnyError>>,
    };
    any.transpose().ok().flatten()
}

#[cfg(any())]
fn to_prost_error_detail(detail: wkt::Any) -> Option<prost_types::Any> {
    use wkt::prost::Convert;
    let url = detail.type_url();
    if url.is_none() {
        return None;
    }
    let any = match url.unwrap() {
        "type.googleapis.com/google.rpc.BadRequest" => detail
            .try_into_message::<rpc::model::BadRequest>()
            .ok()
            .map(|v| v.cnv())
            .as_ref()
            .map(prost_types::Any::from_msg),
        "type.googleapis.com/google.rpc.DebugInfo" => detail
            .try_into_message::<rpc::model::DebugInfo>()
            .ok()
            .map(|v| v.cnv())
            .as_ref()
            .map(prost_types::Any::from_msg),
        "type.googleapis.com/google.rpc.ErrorInfo" => detail
            .try_into_message::<rpc::model::ErrorInfo>()
            .ok()
            .map(|v| v.cnv())
            .as_ref()
            .map(prost_types::Any::from_msg),
        "type.googleapis.com/google.rpc.Help" => detail
            .try_into_message::<rpc::model::Help>()
            .ok()
            .map(|v| v.cnv())
            .as_ref()
            .map(prost_types::Any::from_msg),
        "type.googleapis.com/google.rpc.LocalizedMessage" => detail
            .try_into_message::<rpc::model::LocalizedMessage>()
            .ok()
            .map(|v| v.cnv())
            .as_ref()
            .map(prost_types::Any::from_msg),
        "type.googleapis.com/google.rpc.PreconditionFailure" => detail
            .try_into_message::<rpc::model::PreconditionFailure>()
            .ok()
            .map(|v| v.cnv())
            .as_ref()
            .map(prost_types::Any::from_msg),
        "type.googleapis.com/google.rpc.QuotaFailure" => detail
            .try_into_message::<rpc::model::QuotaFailure>()
            .ok()
            .map(|v| v.cnv())
            .as_ref()
            .map(prost_types::Any::from_msg),
        "type.googleapis.com/google.rpc.RequestInfo" => detail
            .try_into_message::<rpc::model::RequestInfo>()
            .ok()
            .map(|v| v.cnv())
            .as_ref()
            .map(prost_types::Any::from_msg),
        "type.googleapis.com/google.rpc.ResourceInfo" => detail
            .try_into_message::<rpc::model::ResourceInfo>()
            .ok()
            .map(|v| v.cnv())
            .as_ref()
            .map(prost_types::Any::from_msg),
        "type.googleapis.com/google.rpc.RetryInfo" => detail
            .try_into_message::<rpc::model::RetryInfo>()
            .ok()
            .map(|v| v.cnv())
            .as_ref()
            .map(prost_types::Any::from_msg),
        _ => None,
    };
    any.transpose().ok().flatten()
}
