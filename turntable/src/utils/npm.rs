use std::{collections::HashMap, time::Duration};

use bytes::Bytes;
use cached::proc_macro::cached;

use once_cell::sync::Lazy;

use reqwest::StatusCode;
use serde::Deserialize;

use urlencoding::encode;

use crate::extractors::{Entry, PackageConfig};

static REQUEST_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
  reqwest::ClientBuilder::new()
    .tcp_keepalive(Duration::from_secs(1))
    .build()
    .expect("init reqwest client ok!")
});

static NPM_REGISTRY_URL: Lazy<&'static str> =
  Lazy::new(|| option_env!("NPM_REGISTRY_URL").unwrap_or("https:/registry.npmmirror.com"));

#[inline]
fn is_scoped_package_name(pkg_name: impl AsRef<str>) -> bool {
  pkg_name.as_ref().starts_with('@')
}

#[inline]
fn encode_package_name(pkg_name: impl AsRef<str>) -> String {
  let pkg_name: &str = pkg_name.as_ref();
  if is_scoped_package_name(pkg_name) {
    format!("@{}", encode(&pkg_name[1..]))
  } else {
    encode(pkg_name).to_string()
  }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
struct PackageInfo {
  versions: Option<HashMap<String, PackageConfig>>,
  dist_tags: Option<HashMap<String, String>>,
}

async fn fetch_pkg_info(package_name: impl AsRef<str>) -> anyhow::Result<PackageInfo> {
  let package_name = package_name.as_ref();
  let name = encode_package_name(package_name);
  let info_url = format!("{}/{name}", *NPM_REGISTRY_URL);

  tracing::debug!(
    "Fetching package info for {} from {}",
    package_name,
    info_url
  );

  let res = REQUEST_CLIENT.get(info_url).send().await?;
  let code = res.status();
  if code == StatusCode::OK {
    let res = res.json::<PackageInfo>().await?;
    Ok(res)
  } else {
    let content = res.text().await?;
    tracing::error!(
      "Error fetching info for {} (status: {})",
      package_name,
      code
    );
    anyhow::bail!(content)
  }
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionsAndTags {
  pub versions: Vec<String>,
  pub tags: HashMap<String, String>,
}

#[cached(
  size = 200,
  time = 300,
  sync_writes = true,
  result = true,
  key = "String",
  convert = r#"{ format!("versions-{}",package_name.as_ref()) }"#
)]
pub async fn get_versions_and_tags(
  package_name: impl AsRef<str>,
) -> anyhow::Result<VersionsAndTags> {
  let info = fetch_pkg_info(package_name).await?;
  match info.versions {
    Some(versions) => Ok(VersionsAndTags {
      versions: versions
        .keys()
        .map(|k| k.to_owned())
        .collect::<Vec<String>>(),
      tags: info.dist_tags.unwrap_or_default(),
    }),
    None => anyhow::bail!("Get package versions fail"),
  }
}

#[derive(Debug, Clone)]
pub struct SearchEntry {
  pub found_entry: Option<Entry>,
  pub matching_entries: HashMap<String, Entry>,
}

#[cached(
  size = 200,
  time = 300,
  sync_writes = true,
  option = true,
  key = "String",
  convert = r#"{ format!("config-{}-{}",package_name.as_ref(),version.as_ref()) }"#
)]
pub async fn get_package_config(
  package_name: impl AsRef<str>,
  version: impl AsRef<str>,
) -> Option<PackageConfig> {
  let version = version.as_ref();
  match fetch_pkg_info(package_name).await {
    Ok(info) => info
      .versions
      .and_then(|versions| versions.get(version).map(ToOwned::to_owned)),
    Err(_) => None,
  }
}

pub async fn get_package(
  package_name: impl AsRef<str>,
  version: impl AsRef<str>,
) -> anyhow::Result<Bytes> {
  let package_name = package_name.as_ref();
  let version = version.as_ref();
  let tarball_name = if is_scoped_package_name(package_name) {
    package_name.split('/').nth(1).unwrap_or_default()
  } else {
    package_name
  };

  let tarball_url = format!(
    "{}/{package_name}/-/{tarball_name}-{version}.tgz",
    *NPM_REGISTRY_URL
  );

  tracing::debug!("Fetching package for {package_name} from {tarball_url}");

  let resp = reqwest::get(tarball_url).await?;
  let resp = resp.bytes().await?;

  Ok(resp)
}

#[cfg(test)]
mod tests {

  use super::*;

  #[tokio::test]
  async fn test_fetch_pkg_info() -> anyhow::Result<()> {
    fetch_pkg_info("builtins").await?;

    Ok(())
  }

  #[tokio::test]
  async fn test_get_versions_and_tags() -> anyhow::Result<()> {
    let _vt = get_versions_and_tags("builtins").await?;
    let vt = get_versions_and_tags("antd").await?;
    println!("{:?}", vt);
    Ok(())
  }
}
