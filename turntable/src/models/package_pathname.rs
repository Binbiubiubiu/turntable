use std::str::FromStr;

use poem::{FromRequest, Request, RequestBody};
use urlencoding::decode;

use crate::errors::AppError;

#[derive(Debug, Clone, Default)]
pub struct PackagePathname {
  pub package_name: String,
  pub package_version: String,
  pub package_spec: String,
  pub filename: String,
}

#[poem::async_trait]
impl<'a> FromRequest<'a> for &'a PackagePathname {
  async fn from_request(req: &'a Request, _: &mut RequestBody) -> poem::Result<Self> {
    req
      .extensions()
      .get::<PackagePathname>()
      .ok_or(anyhow::anyhow!(
        "get package pathname from the request extensions"
      ))
      .map_err(Into::into)
  }
}

impl FromStr for PackagePathname {
  type Err = AppError;

  fn from_str(pathname: &str) -> poem::Result<Self, Self::Err> {
    let pathname = decode(pathname).map_err(|_| AppError::InvalidURL(pathname.to_owned()))?;

    let package_pathname_format = regex!("^/((?:@[^/@]+/)?[^/@]+)(?:@([^/]+))?(/.*)?$");
    let captures = package_pathname_format.captures(&pathname);

    match captures {
      Some(matched) => {
        let package_name = matched.get(1).map(|s| s.as_str()).unwrap_or_default();
        let package_version = matched.get(2).map(|s| s.as_str()).unwrap_or("latest");
        let filename = matched
          .get(3)
          .map(|s| regex!(r"//+").replace_all(s.as_str(), "/").to_string())
          .unwrap_or_default();

        Ok(PackagePathname {
          package_name: package_name.into(),
          package_version: package_version.into(),
          package_spec: format!("{}@{}", package_name, package_version),
          filename,
        })
      }
      None => Err(AppError::InvalidURL(pathname.to_string())),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_package_pathname() {
    let parsed: PackagePathname = "/@scope/name@version/file.js".parse().unwrap();
    assert_eq!(parsed.package_name, "@scope/name");
    assert_eq!(parsed.package_version, "version");
    assert_eq!(parsed.package_spec, "@scope/name@version");
    assert_eq!(parsed.filename, "/file.js");
  }
}
