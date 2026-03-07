# demo-apphub

This directory contains a demo application demonstrating how to deploy Rust applications to Cloud Run and monitor them with Google Cloud AppHub.

## Building and Deploying

Because this application relies on other crates in the Rust workspace, you must build the Docker image from the root of the workspace.

1. Ensure you are authenticated with Google Cloud:
   ```bash
   gcloud auth login
   gcloud config set project YOUR_PROJECT_ID
   GOOGLE_CLOUD_PROJECT="$(gcloud config get project)"
   ```

2. Build the Docker image using Google Cloud Build (run from the workspace root):
   ```bash
   gcloud builds submit \
     --tag us-central1-docker.pkg.dev/${GOOGLE_CLOUD_PROJECT}/YOUR_REPO_NAME/demo-apphub \
     --file demos/apphub/Dockerfile \
     .
   ```

3. Deploy the built image to Cloud Run:
   ```bash
   gcloud run deploy demo-apphub \
     --image us-central1-docker.pkg.dev/${GOOGLE_CLOUD_PROJECT}/YOUR_REPO_NAME/demo-apphub \
     --allow-unauthenticated \
     --region us-central1
   ```
