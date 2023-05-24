use crate::{
  errors::AppError,
  models::{PackageConfig, PackagePathname, PackageQuery},
  utils::{redirect, url::create_pkg_url},
};
use poem::{
  http::header, Endpoint, FromRequest, IntoResponse, Middleware, Request, Response, Result,
};

pub struct ValidateFilename;

impl<E: Endpoint> Middleware<E> for ValidateFilename {
  type Output = ValidateFilenameEndpoint<E>;

  fn transform(&self, ep: E) -> Self::Output {
    ValidateFilenameEndpoint { ep }
  }
}

pub struct ValidateFilenameEndpoint<E> {
  ep: E,
}

#[poem::async_trait]
impl<E: Endpoint> Endpoint for ValidateFilenameEndpoint<E> {
  type Output = Response;

  async fn call(&self, req: Request) -> Result<Self::Output> {
    let Some(pkg) = req.extensions().get::<PackagePathname>() else {
            return Ok(self.ep.call(req).await?.into_response())
        };

    if pkg.filename.is_empty() {
      return filename_redirect(pkg, &req).await;
    }
    Ok(self.ep.call(req).await?.into_response())
  }
}

async fn filename_redirect(pkg: &PackagePathname, req: &Request) -> Result<Response> {
  let query = PackageQuery::from_request_without_body(req).await?;
  let package_config = <&PackageConfig>::from_request_without_body(req).await?;

  let module_filename = query
    .module
    .is_some()
    .then(|| {
      package_config
        .get_str("module")
        .or(package_config.get_str("jsnext:main"))
        .unwrap_or_default()
    })
    .filter(|filename| filename.is_empty())
    .or_else(|| {
      let ty = package_config.get_str("type");
      match package_config.get_str("main") {
        Some(main) if ty.is_some_and(|ty| ty == "module") => {
          if main.is_empty() {
            Some("/index.js")
          } else {
            Some(main)
          }
        }
        Some(main) if regex!(r".mjs$").is_match(main) => Some(main),
        _ => None,
      }
    })
    .ok_or_else(|| {
      std::convert::Into::<poem::Error>::into(AppError::NotContainEsModule(
        pkg.package_spec.to_owned(),
      ))
    });

  let filename = module_filename.or_else(|_| {
    // other filename
    query
      .main
      .as_ref()
      .and_then(|main| package_config.get_str(main))
      .or_else(|| package_config.get_str("unpkg"))
      .or_else(|| package_config.get_str("browser"))
      .or_else(|| package_config.get_str("main").or(Some("/index.js")))
      .ok_or_else(|| anyhow::anyhow!("get filename from package config. default: \"/index.js\""))
  })?;

  let path = create_pkg_url(
    &pkg.package_name,
    &pkg.package_version,
    regex!(r"^[./]*").replace(filename, "/"),
    req.uri().query(),
  );
  let resp = redirect(path)
    .with_header(header::CACHE_CONTROL, "public, s-maxage=600, max-age=60")
    .with_header("Cache-Tag", "redirect, filename-redirect")
    .into_response();
  Ok(resp)
}
