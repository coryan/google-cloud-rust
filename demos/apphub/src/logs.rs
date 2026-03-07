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

use opentelemetry::TraceId;
use serde::Serializer;
use serde::ser::SerializeMap;
use std::fmt::Result as FmtResult;
use tracing::{Event, Subscriber};
use tracing_opentelemetry::OtelData;
use tracing_serde::AsSerde;
use tracing_serde::fields::AsMap;
use tracing_subscriber::fmt::format::{FormatEvent, Writer};
use tracing_subscriber::fmt::{FmtContext, FormatFields};
use tracing_subscriber::registry::LookupSpan;

pub struct EventFormatter;
impl<S, N> FormatEvent<S, N> for EventFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> FmtResult
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
        N: for<'a> FormatFields<'a> + 'static,
    {
        use serde_json::Serializer;
        let meta = event.metadata();

        let mut visit = || {
            let mut serializer = Serializer::new(WriteAdaptor::new(&mut writer));
            let mut serializer = serializer.serialize_map(None)?;
            serializer.serialize_entry("timestamp", &chrono::Utc::now().to_rfc3339())?;
            serializer.serialize_entry("severity", &meta.level().as_serde())?;
            serializer.serialize_entry("fields", &event.field_map())?;
            serializer.serialize_entry("target", meta.target())?;

            if let Some(span) = ctx.lookup_current() {
                if let Some(data) = span.extensions().get::<OtelData>() {
                    if let Some(tid) = data.trace_id() {
                        if tid != TraceId::INVALID {
                            serializer.serialize_entry(
                                "logging.googleapis.com/trace",
                                &tid.to_string(),
                            )?;
                            serializer
                                .serialize_entry("logging.googleapis.com/trace_sampled", &true)?;
                        } else {
                            serializer
                                .serialize_entry("logging.googleapis.com/trace_sampled", &false)?;
                        }
                    }
                    if let Some(sid) = data.span_id() {
                        serializer
                            .serialize_entry("logging.googleapis.com/span", &sid.to_string())?;
                    }
                    // if let Some(sampled) = span.span span.span_context().is_valid() {
                    //     log_record["logging.googleapis.com/trace_sampled"] = json!(sampled);
                    // }
                }
            }

            serializer.end()
        };
        visit().map_err(|_| std::fmt::Error)?;
        writeln!(writer)
    }
}

/// Make a `std::fmt::write` look like a `std::io::Write` so we can use it as the destination of a
/// `serde_json::Serializer`.
struct WriteAdaptor<'a> {
    fmt_write: &'a mut dyn std::fmt::Write,
}

impl<'a> WriteAdaptor<'a> {
    pub fn new(fmt_write: &'a mut dyn std::fmt::Write) -> Self {
        Self { fmt_write }
    }
}

impl<'a> std::io::Write for WriteAdaptor<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let s = std::str::from_utf8(buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        self.fmt_write
            .write_str(s)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        Ok(s.as_bytes().len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
