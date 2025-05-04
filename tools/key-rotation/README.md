# Automatically rotate keys for the integration tests

This directory contains a program to automatically rotate the keys used in the
`auth` integration tests. The program is deployed to Cloud Run, and invoked
daily using Cloud Scheduler.


## Deployment

```shell
GOOGLE_CLOUD_PROJECT="$(gcloud config get project)"
gcloud run deploy key-rotation --source=tools/key-rotation --region=us-central1 \
    --set-env-vars=GOOGLE_CLOUD_PROJECT=${GOOGLE_CLOUD_PROJECT},SERVICE_ACCOUNT=test-sa-creds-json,SECRET_ID=test-sa-creds-secret
```
