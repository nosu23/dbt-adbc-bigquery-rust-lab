use std::env;
use std::error::Error;
use std::io;
use std::path::PathBuf;

use adbc_core::options::{AdbcVersion, OptionDatabase, OptionStatement};
use adbc_core::{Connection, Database, Driver, Optionable, Statement};
use adbc_driver_manager::ManagedDriver;
use arrow_array::StringArray;

const GET_DATASET_METADATA: &str = "adbc.bigquery.dataset.get_metadata";
const PROJECT_ID_OPTION: &str = "adbc.bigquery.sql.project_id";
const DATASET_ID_OPTION: &str = "adbc.bigquery.sql.dataset_id";
const AUTH_TYPE_OPTION: &str = "adbc.bigquery.sql.auth_type";
const APP_DEFAULT_CREDENTIALS: &str = "adbc.bigquery.sql.auth_type.app_default_credentials";
const IMPERSONATE_TARGET_PRINCIPAL_OPTION: &str = "adbc.bigquery.sql.impersonate.target_principal";
const IMPERSONATE_SCOPES_OPTION: &str = "adbc.bigquery.sql.impersonate.scopes";
const IMPERSONATE_SCOPES: &str = concat!(
    "https://www.googleapis.com/auth/bigquery,",
    "https://www.googleapis.com/auth/cloud-platform,",
    "https://www.googleapis.com/auth/drive,",
    "https://www.googleapis.com/auth/userinfo.email"
);
const DEFAULT_DRIVER_RELATIVE_PATH: &str =
    "../../driver/arrow-adbc/go/adbc/pkg/libadbc_driver_bigquery.dylib";

struct Config {
    project_id: String,
    dataset_id: String,
    driver_path: PathBuf,
    impersonate_service_account: Option<String>,
}

impl Config {
    fn from_args() -> Result<Self, Box<dyn Error>> {
        let mut project_id = env::var("GOOGLE_CLOUD_PROJECT").ok();
        let mut dataset_id = None;
        let mut driver_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_DRIVER_RELATIVE_PATH);
        let mut impersonate_service_account = None;
        let mut args = env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--project" => project_id = Some(Self::next_value(&mut args, "--project")?),
                "--dataset" => dataset_id = Some(Self::next_value(&mut args, "--dataset")?),
                "--driver" => driver_path = PathBuf::from(Self::next_value(&mut args, "--driver")?),
                "--impersonate-service-account" => {
                    impersonate_service_account = Some(Self::next_value(
                        &mut args,
                        "--impersonate-service-account",
                    )?)
                }
                "-h" | "--help" => {
                    println!("{}", Self::usage());
                    std::process::exit(0);
                }
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("unknown argument: {arg}\n\n{}", Self::usage()),
                    )
                    .into());
                }
            }
        }

        let project_id = project_id.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("project ID is required\n\n{}", Self::usage()),
            )
        })?;
        let dataset_id = dataset_id.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("dataset ID is required\n\n{}", Self::usage()),
            )
        })?;

        Ok(Self {
            project_id,
            dataset_id,
            driver_path,
            impersonate_service_account,
        })
    }

    fn next_value(
        args: &mut impl Iterator<Item = String>,
        option: &str,
    ) -> Result<String, io::Error> {
        args.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{option} requires a value"),
            )
        })
    }

    fn usage() -> &'static str {
        "Usage:\n  cargo run --manifest-path rust_lab/get_dataset_metadata/Cargo.toml -- --project PROJECT_ID --dataset DATASET_ID [--impersonate-service-account EMAIL] [--driver PATH]\n\n\
         GOOGLE_CLOUD_PROJECT may be used instead of --project."
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::from_args()?;
    if !config.driver_path.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "BigQuery ADBC driver not found: {}",
                config.driver_path.display()
            ),
        )
        .into());
    }

    let mut driver =
        ManagedDriver::load_dynamic_from_filename(&config.driver_path, None, AdbcVersion::V110)?;
    let mut database_options = vec![
        (
            OptionDatabase::Other(PROJECT_ID_OPTION.to_owned()),
            config.project_id.clone().into(),
        ),
        (
            OptionDatabase::Other(DATASET_ID_OPTION.to_owned()),
            config.dataset_id.clone().into(),
        ),
        (
            OptionDatabase::Other(AUTH_TYPE_OPTION.to_owned()),
            APP_DEFAULT_CREDENTIALS.into(),
        ),
    ];
    if let Some(service_account) = &config.impersonate_service_account {
        database_options.push((
            OptionDatabase::Other(IMPERSONATE_TARGET_PRINCIPAL_OPTION.to_owned()),
            service_account.clone().into(),
        ));
        database_options.push((
            OptionDatabase::Other(IMPERSONATE_SCOPES_OPTION.to_owned()),
            IMPERSONATE_SCOPES.into(),
        ));
    }

    let database = driver.new_database_with_opts(database_options)?;
    let mut connection = database.new_connection()?;
    let mut statement = connection.new_statement()?;
    let request = serde_json::json!({
        "project": config.project_id,
        "dataset": config.dataset_id,
    });
    statement.set_option(
        OptionStatement::Other(GET_DATASET_METADATA.to_owned()),
        request.to_string().into(),
    )?;

    let mut found = false;
    for batch in statement.execute()? {
        let batch = batch?;
        let metadata = batch
            .column_by_name("metadata")
            .and_then(|column| column.as_any().downcast_ref::<StringArray>())
            .ok_or_else(|| io::Error::other("driver result has no metadata string column"))?;
        for row in 0..batch.num_rows() {
            let value: serde_json::Value = serde_json::from_str(metadata.value(row))?;
            println!("{}", serde_json::to_string_pretty(&value)?);
            found = true;
        }
    }
    if !found {
        return Err(io::Error::other("driver returned no dataset metadata").into());
    }
    Ok(())
}
