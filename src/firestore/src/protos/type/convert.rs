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

impl std::convert::From<gtype::model::LatLng> for LatLng {
    fn from(value: gtype::model::LatLng) -> Self {
        Self {
            latitude: value.latitude.into(),
            longitude: value.longitude.into(),
        }
    }
}

impl std::convert::From<LatLng> for gtype::model::LatLng {
    fn from(value: LatLng) -> Self {
        Self::new().set_latitude(value.latitude).set_longitude(value.longitude)
    }
}
