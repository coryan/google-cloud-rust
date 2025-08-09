# List-Batch-Delete Benchmark

Benchmarks the Cloud Storage client library. The benchmark lists objects and
deletes them in batches, reporting the latency for the delete batches.

## Pre-requisites

Obtain a GCE instance. You should try to use a \[Compute-optimized\] instance
(e.g. the `c2d-*` family).

## Bucket

Use a bucket in the same region as your VM. If you need to create a bucket,
these instructions may help:

- Create a configuration file to automatically delete objects after one day

  ```shell
  echo '{ "lifecycle": { "rule": [ { "action": {"type": "Delete"}, "condition": {"age": 1} } ] } }' > lf.json
  ```

- Create the bucket. Replace the `${REGION}` and `${BUCKET_NAME}` placeholders
  as needed:

  ```shell
  gcloud storage buckets create \
    --enable-hierarchical-namespace --uniform-bucket-level-access \
    --soft-delete-duration=0s --lifecycle-file=lf.json \
    --location=${REGION}  gs://${BUCKET_NAME}
  ```

## Running

```shell
TS=$(date +%s); cargo run --release --package storage-lbd -- \
    --bucket-name ${BUCKET_NAME} \
    --min-sample-count=1000  >bm-${TS}.txt 2>bm-${TS}.log </dev/null &
```

## Load data to BigQuer

```shell
bq load --skip_leading_rows=1 lbd.test01 bm-${TS}.txt \
    "Iteration:int64,TargetSize:int64,BatchSize:int64,ElapsedMicroseconds:int64,RelativeMicroseconds:int64,ErrorCount:int64"
```
