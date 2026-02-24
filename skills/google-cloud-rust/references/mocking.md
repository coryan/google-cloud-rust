# Testing & Mocking Clients

Applications can use mocks to write controlled unit tests without network calls or billing using the `mockall` framework.

1. Add `mockall` to `[dev-dependencies]`.

## Setting up a Mock Client

Implement the generated client `stub` trait (e.g., `speech::stub::Speech`) using the `mockall::mock!` macro.

```rust
use google_cloud_speech_v2::client::Speech;
use google_cloud_speech_v2::model::{RecognizeRequest, RecognizeResponse};
use google_cloud_speech_v2::stub::Speech as SpeechStub; // Import the stub trait
use google_cloud_gax::error::Error;
use std::sync::Arc;

// Define the mock struct
mockall::mock! {
    pub SpeechStub {}
    
    #[async_trait::async_trait]
    impl SpeechStub for SpeechStub {
        async fn recognize(&self, req: RecognizeRequest) -> Result<RecognizeResponse, Error>;
        // Mock other methods as needed
    }
}

#[tokio::test]
async fn test_recognize_success() {
    let mut mock = MockSpeechStub::new();

    // Set expectations
    mock.expect_recognize()
        .withf(|req| req.recognizer == "projects/p/locations/l/recognizers/r")
        .times(1)
        .returning(|_| {
            Ok(RecognizeResponse {
                results: vec![],
                // ...
                ..Default::default()
            })
        });

    // Create a client from the stub
    let client = Speech::from_stub(Arc::new(mock));
    
    // Use the client in your application function...
}
```