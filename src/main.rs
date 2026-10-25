use std::env;
use std::error::Error;
use std::io;
use std::path::PathBuf;

use adbc_core::options::{AdbcVersion, OptionDatabase};
use adbc_core::{Connection, Database, Driver, Statement};
use adbc_driver_manager::ManagedDriver;
use arrow::util::pretty::print_batches;

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
    "driver/arrow-adbc/go/adbc/pkg/libadbc_driver_bigquery.dylib";
const DEFAULT_QUERY: &str = "SELECT 42 AS answer";

#[derive(Debug)]
struct Config {
    dataset_id: String,
    driver_path: PathBuf,
    impersonate_service_account: Option<String>,
    project_id: String,
    query: String,
}

impl Config {
    fn from_args() -> Result<Self, Box<dyn Error>> {
        let mut dataset_id = None;
        let mut driver_path = Self::default_driver_path();
        let mut impersonate_service_account = None;
        let mut project_id = env::var("GOOGLE_CLOUD_PROJECT").ok();
        let mut query = DEFAULT_QUERY.to_owned();
        let mut args = env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--dataset" => dataset_id = Some(Self::next_value(&mut args, "--dataset")?),
                "--driver" => driver_path = PathBuf::from(Self::next_value(&mut args, "--driver")?),
                "--impersonate-service-account" => {
                    impersonate_service_account = Some(Self::next_value(
                        &mut args,
                        "--impersonate-service-account",
                    )?)
                }
                "--project" => project_id = Some(Self::next_value(&mut args, "--project")?),
                "--query" => query = Self::next_value(&mut args, "--query")?,
                "-h" | "--help" => {
                    Self::print_usage();
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
                format!(
                    "BigQuery project ID is required; pass --project or set \
                     GOOGLE_CLOUD_PROJECT\n\n{}",
                    Self::usage()
                ),
            )
        })?;

        let dataset_id = dataset_id.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "BigQuery dataset ID is required; pass --dataset DATASET_ID\n\n{}",
                    Self::usage()
                ),
            )
        })?;

        Ok(Self {
            dataset_id,
            driver_path,
            impersonate_service_account,
            project_id,
            query,
        })
    }

    fn default_driver_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_DRIVER_RELATIVE_PATH)
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

    fn print_usage() {
        println!("{}", Self::usage());
    }

    fn usage() -> &'static str {
        "Usage:\n  cargo run -- --project PROJECT_ID --dataset DATASET_ID [--impersonate-service-account EMAIL] [--query SQL] [--driver PATH]\n\n\
         Options:\n  --project PROJECT_ID                 BigQuery billing/project ID\n  --dataset DATASET_ID                 BigQuery default dataset ID\n  --impersonate-service-account EMAIL  Service account to impersonate\n  --query SQL                           SQL to execute (default: SELECT 42 AS answer)\n  --driver PATH                         BigQuery ADBC dynamic library path\n  -h, --help                            Show this help\n\n\
         GOOGLE_CLOUD_PROJECT may be used instead of --project."
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::from_args()?;

    if !config.driver_path.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "BigQuery ADBC driver not found: {}\nBuild it first with the command in README.md.",
                config.driver_path.display()
            ),
        )
        .into());
    }

    println!("Driver : {}", config.driver_path.display());
    println!("Project: {}", config.project_id);
    println!("Dataset: {}", config.dataset_id);
    if let Some(service_account) = &config.impersonate_service_account {
        println!("Impersonate: {service_account}");
    }
    println!("Query  : {}", config.query);

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
    statement.set_sql_query(&config.query)?;

    let batches = statement.execute()?.collect::<Result<Vec<_>, _>>()?;
    print_batches(&batches)?;

    Ok(())
}
