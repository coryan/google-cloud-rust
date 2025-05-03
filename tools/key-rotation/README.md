# Automatically rotate keys for the integration tests

This directory contains a program to automatically rotate the keys used in the
`auth` integration tests. The program is deployed to Cloud Run, and invoked
daily using Cloud Scheduler.


## Deployment

```shell
gcloud run deploy key-rotation --source=tools/key-rotation --region=us-central1
```
