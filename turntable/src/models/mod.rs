mod entry;
mod metadata;
mod package_config;
mod package_pathname;

use chrono::{DateTime, NaiveDateTime, Utc};
pub use entry::*;
pub use metadata::*;
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

pub struct Mtime(u64);

impl TryFrom<Mtime> for DateTime<Utc> {
  type Error = anyhow::Error;

  fn try_from(value: Mtime) -> Result<Self, Self::Error> {
    let timestamp = NaiveDateTime::from_timestamp_opt(value.0.try_into()?, 0);
    match timestamp {
      Some(timestamp) => Ok(DateTime::<Utc>::from_utc(timestamp, Utc)),
      None => anyhow::bail!("mtime convert to datetime utc error"),
    }
  }
}

impl From<u64> for Mtime {
  fn from(value: u64) -> Self {
    Self(value)
  }
}
