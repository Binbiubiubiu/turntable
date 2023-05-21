mod entry;
mod package_config;
mod package_pathname;

pub use entry::*;
pub use package_config::*;
pub use package_pathname::*;
use poem::web::Query;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct OptionInQuery {
  pub module: Option<String>,
  pub meta: Option<String>,
  pub main: Option<String>,
}

pub type PackageQuery = Query<OptionInQuery>;
