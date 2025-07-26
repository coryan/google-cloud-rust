<!-- 
Copyright 2025 Google LLC

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    https://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
-->

# Speed up large object downloads

In this tutorial you will learn how to speed up downloads of large objects from
[Cloud Storage] by striping the download across multiple sections of the object.

## Prerequisites

The guide assumes you have an existing [Google Cloud project] with
[billing enabled], and a Cloud Storage bucket in that project.

You will create some large objects during this tutorial, remember to clean up
any resources to avoid excessive billing.

The tutorial assumes you are familiar with the basics of using the client
library, read the [quickstart guide]

## Add the client library as a dependency

```toml
{{#include ../../samples/Cargo.toml:storage}}
```

## Create source data

To run this tutorial you will need some large objects in Cloud Storage. You
can create such objects by seeding a smaller object and then repeatedly compose
it to create objects of the desired size.

```rust,ignore,noplayground
{{#rustdoc_include ../../samples/tests/storage/striped.rs:seed-data}}
```

[quickstart guide]: /storage.md#quickstart
[billing enabled]: https://cloud.google.com/billing/docs/how-to/verify-billing-enabled#confirm_billing_is_enabled_on_a_project
[cloud storage]: https://cloud.google.com/storage
[google cloud project]: https://cloud.google.com/resource-manager/docs/creating-managing-projects
