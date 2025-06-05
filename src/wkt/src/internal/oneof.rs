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

//! Provide a helper to detect overlapping `oneof` entries.

use std::marker::PhantomData;

pub struct OnlyOne<T>(PhantomData<T>);

impl<'de, T> serde_with::DeserializeAs<'de, Option<T>> for OnlyOne<T>
where
    T: serde::de::Deserialize<'de> + std::fmt::Debug,
{
    fn deserialize_as<D>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        use serde::de::{Deserialize, Error};
        let vec = Vec::<T>::deserialize(deserializer);
        println!("  ### {vec:?}");
        match vec? {
            v if v.is_empty() => Ok(None),
            mut v if v.len() == 1 => {
                println!("   ###  {v:?}");

                Ok(v.pop())
            }
            _ => Err(D::Error::custom(
                "invalid entry: found multiple values for the same oneof",
            )),
        }
    }
}

impl<'de, T> serde_with::SerializeAs<Option<T>> for OnlyOne<T>
where
    T: serde::ser::Serialize + std::fmt::Debug,
{
    fn serialize_as<S>(source: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::Serialize;
        source.serialize(serializer)
    }
}
