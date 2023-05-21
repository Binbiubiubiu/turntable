use node_semver::{Range, Version};
use poem::{
  http::header, Endpoint, FromRequest, IntoResponse, Middleware, Request, Response, Result,
};

use crate::{
  errors::AppError,
  extractors::PackagePathname,
  utils::{
    npm::{get_package_config, get_versions_and_tags, VersionsAndTags},
    redirect,
    url::create_pkg_url,
  },
};

#[inline]
fn max_satisfies(versions: Vec<String>, range: Range) -> anyhow::Result<Option<String>> {
  let mut max = None;
  let mut max_version = None;
  for v in versions.iter() {
    let vs = Version::parse(v)?;
    if range.satisfies(&vs) && (max.is_none() || max_version.as_ref().is_some_and(|max| *max < vs))
    {
      max = Some(v.to_owned());
      max_version = Some(Version::parse(v)?);
    }
  }
  Ok(max)
}

async fn resolve_version(
  package_name: impl AsRef<str>,
  package_version: impl Into<String>,
) -> anyhow::Result<Option<String>> {
  let package_version = package_version.into();
  let VersionsAndTags { versions, tags } = get_versions_and_tags(package_name).await?;
  let package_version = tags.get(&package_version).unwrap_or(&package_version);

  match versions.contains(package_version) {
    true => Ok(Some(package_version.to_owned())),
    false => max_satisfies(versions, Range::parse(package_version)?),
  }
}

pub struct ValidatePackageVersion;

impl<E: Endpoint> Middleware<E> for ValidatePackageVersion {
  type Output = ValidatePackageVersionEndpoint<E>;

  fn transform(&self, ep: E) -> Self::Output {
    ValidatePackageVersionEndpoint { ep }
  }
}

pub struct ValidatePackageVersionEndpoint<E> {
  ep: E,
}

#[poem::async_trait]
impl<E: Endpoint> Endpoint for ValidatePackageVersionEndpoint<E> {
  type Output = Response;

  async fn call(&self, mut req: Request) -> Result<Self::Output> {
    let pkg = <&PackagePathname>::from_request_without_body(&req).await?;

    let version = resolve_version(&pkg.package_name, &pkg.package_version).await?;

    let Some(version) = version else{
                return Err(AppError::NotFoundPackage(pkg.package_spec.to_owned())).map_err(Into::into);
            };

    if version != pkg.package_version {
      let path = create_pkg_url(&pkg.package_name, version, &pkg.filename, req.uri().query());
      let resp = redirect(path)
        .with_header(header::CACHE_CONTROL, "public, s-maxage=600, max-age=60")
        .with_header("Cache-Tag", "redirect, semver-redirect")
        .into_response();
      return Ok(resp);
    }

    let Some(config) =  get_package_config(&pkg.package_name, &pkg.package_version).await else {
                return Err(AppError::UnableGetConfigForPackage(
                    pkg.package_spec.to_owned(),
                ))
                .map_err(Into::<poem::Error>::into);
            };

    req.extensions_mut().insert(config);

    Ok(self.ep.call(req).await?.into_response())
  }
}
