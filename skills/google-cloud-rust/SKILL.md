---
name: google-cloud-rust
description: Guidelines and workflows for users building applications with the google-cloud-rust crates. Use when writing Rust code that uses Google Cloud client libraries, to handle initialization, storage, or errors.
---

# Google Cloud Rust Client Libraries

This skill provides essential guidelines and patterns for developers using the `google-cloud-rust` crates. It covers common workflows such as initializing clients, using Cloud Storage idiomatics, and handling errors.

## Philosophy

- **Clear is better than clever.** Write simple, boring, readable code.
- **Name length corresponds to scope size.**
- **RPCs are Async:** All remote procedure calls (RPCs) are asynchronous and designed to work with the Tokio runtime.

## Core Workflows

Choose the appropriate reference guide for your task:

- **Client Setup & Configuration**: See [references/client-setup.md](references/client-setup.md) for how to initialize clients and authenticate.
- **Cloud Storage Operations**: See [references/storage.md](references/storage.md) for idiomatic bucket and object operations, including specific crate quirks.
- **Error Handling**: See [references/errors.md](references/errors.md) for parsing service errors and retrying logic.