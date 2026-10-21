use std::env;
use std::error::Error;
use std::io;
use std::path::PathBuf;

use adbc_core::options::{AdbcVersion, OptionDatabase};
use adbc_core::{Connection, Database, Driver, Statement};
use adbc_driver_manager::ManagedDriver;
use arrow::util::pretty::print_batches;

const PROJECT_ID_OPTION: &str = "adbc.bigquery.sql.project_id";
const AUTH_TYPE_OPTION: &str = "adbc.bigquery.sql.auth_type";
const APP_DEFAULT_CREDENTIALS: &str = "adbc.bigquery.sql.auth_type.app_default_credentials";
const DEFAULT_QUERY: &str = "SELECT 42 AS answer";

#[derive(Debug)]
struct Config {
    driver_path: PathBuf,
    project_id: String,
    query: String,
}

impl Config {
    fn from_args() -> Result<Self, Box<dyn Error>> {
        let mut driver_path = default_driver_path();
        let mut project_id = env::var("GOOGLE_CLOUD_PROJECT").ok();
        let mut query = DEFAULT_QUERY.to_owned();
        let mut args = env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--driver" => driver_path = PathBuf::from(next_value(&mut args, "--driver")?),
                "--project" => project_id = Some(next_value(&mut args, "--project")?),
                "--query" => query = next_value(&mut args, "--query")?,
                "-h" | "--help" => {
                    print_usage();
                    std::process::exit(0);
                }
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("unknown argument: {arg}\n\n{}", usage()),
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
                    usage()
                ),
            )
        })?;

        Ok(Self {
            driver_path,
            project_id,
            query,
        })
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
    println!("Query  : {}", config.query);

    let mut driver =
        ManagedDriver::load_dynamic_from_filename(&config.driver_path, None, AdbcVersion::V110)?;

    let database_options = [
        (
            OptionDatabase::Other(PROJECT_ID_OPTION.to_owned()),
            config.project_id.into(),
        ),
        (
            OptionDatabase::Other(AUTH_TYPE_OPTION.to_owned()),
            APP_DEFAULT_CREDENTIALS.into(),
        ),
    ];
    let database = driver.new_database_with_opts(database_options)?;
    let mut connection = database.new_connection()?;
    let mut statement = connection.new_statement()?;
    statement.set_sql_query(&config.query)?;

    let batches = statement.execute()?.collect::<Result<Vec<_>, _>>()?;
    print_batches(&batches)?;

    Ok(())
}

fn default_driver_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("driver/arrow-adbc/go/adbc/pkg/libadbc_driver_bigquery.dylib")
}

fn next_value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, io::Error> {
    args.next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{option} requires a value"),
        )
    })
}

fn print_usage() {
    println!("{}", usage());
}

fn usage() -> &'static str {
    "Usage:\n  cargo run -- --project PROJECT_ID [--query SQL] [--driver PATH]\n\n\
     Options:\n  --project PROJECT_ID  BigQuery billing/project ID\n  --query SQL            SQL to execute (default: SELECT 42 AS answer)\n  --driver PATH          BigQuery ADBC dynamic library path\n  -h, --help             Show this help\n\n\
     GOOGLE_CLOUD_PROJECT may be used instead of --project."
}
