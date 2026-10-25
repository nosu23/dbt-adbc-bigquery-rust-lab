# Get BigQuery dataset metadata

This is a separate Cargo project. The SQL query experiment is in `rust_lab/simple_bq_select/`.

From the repository root, rebuild the modified Go ADBC driver first:

```sh
make -C driver/arrow-adbc/go/adbc/pkg -B \
  BUILD_TAGS=driverlib,netgo,http2legacy \
  libadbc_driver_bigquery.dylib
```

Then run this experiment with Application Default Credentials:

```sh
cargo run --manifest-path rust_lab/get_dataset_metadata/Cargo.toml -- \
  --project PROJECT_ID \
  --dataset DATASET_ID
```

To use service account impersonation, add
`--impersonate-service-account SERVICE_ACCOUNT_EMAIL`. To load another build,
add `--driver PATH`.

The program calls the driver's `adbc.bigquery.dataset.get_metadata` option and
prints its JSON result (`name`, `description`, and `location`). It does not run a
SQL query. The driver must contain the uncommitted `get_dataset_metadata` change;
the previous `.dylib` does not support this option.
