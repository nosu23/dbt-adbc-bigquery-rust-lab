# dbt-adbc-bigquery-rust-lab

Small Rust application for loading a locally built BigQuery ADBC driver from
the dbt Labs fork of `arrow-adbc` and executing a query against BigQuery.

## Prerequisites

- Rust and Cargo
- Go (pinned with `mise.toml`)
- Google Cloud Application Default Credentials (ADC)
- A Google Cloud project with the BigQuery API enabled

Initialize the submodule after cloning:

```sh
git submodule update --init --recursive
```

Install the configured Go toolchain:

```sh
mise install
```

## Build the BigQuery driver

On macOS, build the Go driver as a dynamic library:

```sh
make \
  -C driver/arrow-adbc/go/adbc/pkg \
  -B \
  BUILD_TAGS=driverlib,netgo,http2legacy \
  libadbc_driver_bigquery.dylib
```

The output is:

```text
driver/arrow-adbc/go/adbc/pkg/libadbc_driver_bigquery.dylib
```

`http2legacy` is currently required when building this driver dependency set
with Go 1.27.

## Authenticate

Create local Application Default Credentials:

```sh
gcloud auth application-default login
```

Optionally set the quota project:

```sh
gcloud auth application-default set-quota-project PROJECT_ID
```

## Run a query

The default query is `SELECT 42 AS answer`:

```sh
cargo run -- --project PROJECT_ID
```

Run a custom query:

```sh
cargo run -- \
  --project PROJECT_ID \
  --query 'SELECT CURRENT_TIMESTAMP() AS current_time'
```

`GOOGLE_CLOUD_PROJECT` can be used instead of `--project`:

```sh
export GOOGLE_CLOUD_PROJECT=PROJECT_ID
cargo run
```

To load a driver from another location, pass `--driver PATH`.
