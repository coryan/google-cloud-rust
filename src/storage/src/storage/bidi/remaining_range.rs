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

use super::normalized_range::NormalizedRange;
use crate::error::ReadError;
use crate::google::storage::v2::ReadRange as ProtoRange;
use crate::model_ext::RequestedRange;

type ReadResult<T> = std::result::Result<T, ReadError>;

/// Tracks the remaining range.
///
/// [PendingRange][super::pending_range::PendingRange] is initialized with
/// the requested ranges. These are normalized when the first response arrives.
/// Both the normalized and initial ranges must be usable to resume connections.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RemainingRange {
    Requested(RequestedRange),
    Normalized(NormalizedRange),
}

impl RemainingRange {
    pub fn update(&mut self, response: ProtoRange) -> ReadResult<()> {
        match self {
            Self::Normalized(segment) => segment.update(response)?,
            Self::Requested(range) => {
                let mut segment = Self::normalize(*range, response)?;
                segment.update(response)?;
                *self = Self::Normalized(segment);
            }
        };
        Ok(())
    }

    fn normalize(current: RequestedRange, response: ProtoRange) -> ReadResult<NormalizedRange> {
        match current {
            RequestedRange::Tail(_) => NormalizedRange::new(response.read_offset),

            RequestedRange::Offset(offset) if response.read_offset as u64 != offset => {
                Err(ReadError::OutOfOrderBidiResponse {
                    got: response.read_offset,
                    expected: offset as i64,
                })
            }
            RequestedRange::Offset(_) => NormalizedRange::new(response.read_offset),

            RequestedRange::Segment { limit, .. }
                if response.read_length as u64 > limit && limit != 0 =>
            {
                Err(ReadError::LongRead {
                    got: response.read_length as u64,
                    expected: limit,
                })
            }
            RequestedRange::Segment { offset, .. } if response.read_offset as u64 != offset => {
                Err(ReadError::OutOfOrderBidiResponse {
                    got: response.read_offset,
                    expected: offset as i64,
                })
            }
            RequestedRange::Segment { limit: 0_u64, .. } => {
                NormalizedRange::new(response.read_offset)
            }
            RequestedRange::Segment { limit, .. } => NormalizedRange::new(response.read_offset)?
                .with_length(limit.clamp(0, i64::MAX as u64) as i64),
        }
    }

    pub fn as_proto(&self, id: i64) -> ProtoRange {
        match self {
            Self::Requested(r) => r.as_proto(id),
            Self::Normalized(s) => s.as_proto(id),
        }
    }

    pub fn handle_empty(&self, end: bool) -> ReadResult<()> {
        match self {
            Self::Normalized(s) => s.handle_empty(end),
            Self::Requested(_) => unreachable!("always called after update()"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::proto_range;
    use super::*;
    use crate::model_ext::ReadRange;
    use test_case::test_case;

    #[test_case(ReadRange::all(), proto_range(0, 100), proto_range(100, 0))]
    #[test_case(ReadRange::offset(1000), proto_range(1000, 100), proto_range(1100, 0))]
    #[test_case(ReadRange::tail(1000), proto_range(2000, 100), proto_range(2100, 0))]
    #[test_case(ReadRange::head(1000), proto_range(0, 100), proto_range(100, 900))]
    #[test_case(
        ReadRange::segment(1000, 2000),
        proto_range(1000, 100),
        proto_range(1100, 1900)
    )]
    fn initial_update(
        input: ReadRange,
        update: ProtoRange,
        want: ProtoRange,
    ) -> anyhow::Result<()> {
        let mut remaining = RemainingRange::Requested(input.0);
        remaining.update(update)?;
        assert_eq!(remaining.as_proto(0), want, "{remaining:?}");
        Ok(())
    }
}
