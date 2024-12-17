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

//! Implements a fake server for [crate::traits::FooService].

use axum::{http::StatusCode, Json};
use gax::error::HttpError;
use std::collections::HashMap;
use tokio::task::JoinHandle;

type Error = gax::error::HttpError;
type HandlerResult<T> = std::result::Result<T, Error>;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

use std::sync::Arc;
use std::sync::Mutex;

type SharedState = Arc<Mutex<State>>;

#[derive(Default)]
struct State {
    foos: HashMap<String, crate::model::Foo>,
}

pub async fn start() -> Result<(String, JoinHandle<()>)> {
    let state = Arc::new(Mutex::new(State::default()));

    let app = axum::Router::new();
    let app = app
        .route("/v0/foos", axum::routing::get(list))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await?;
    let addr = listener.local_addr()?;
    let server = tokio::spawn(async {
        axum::serve(listener, app).await.unwrap();
    });

    Ok((format!("http://{}:{}", addr.ip(), addr.port()), server))
}

fn to_internal_error(e: impl std::error::Error) -> HttpError {
    let payload = format!("{e}");
    let payload = bytes::Bytes::from_owner(payload);
    HttpError::new(
        StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
        HashMap::default(),
        payload.into(),
    )
}

async fn list(
    axum::extract::Query(query): axum::extract::Query<HashMap<String, String>>,
    axum::extract::State(state): axum::extract::State<SharedState>,
) -> (StatusCode, Json<serde_json::Value>) {
    match list_impl(query, state) {
        Ok(value) => (StatusCode::OK, Json(value)),
        Err(e) => {
            let code =
                StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            // TODO - convert the error payload
            (code, Json(serde_json::Value::default()))
        }
    }
}

fn list_impl(
    query: HashMap<String, String>,
    state: SharedState,
) -> HandlerResult<serde_json::Value> {
    use crate::model::*;
    let filter: Box<dyn Fn(&Foo) -> bool> = if let Some(prefix) = query.get("prefix") {
        let p = prefix.to_owned();
        Box::new(move |item: &crate::model::Foo| item.name.starts_with(&p))
    } else {
        Box::new(|_: &crate::model::Foo| true)
    };

    let state = state.lock().map_err(to_internal_error)?;
    let response = ListFoosResponse {
        items: state
            .foos
            .iter()
            .filter_map(|(_, v)| filter(v).then_some(v.clone()))
            .collect(),
        next_page_token: None, //
    };
    serde_json::to_value(response).map_err(to_internal_error)
}
