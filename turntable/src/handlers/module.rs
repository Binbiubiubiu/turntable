use mime_guess::mime;
use poem::{
  http::{header, StatusCode},
  FromRequest, IntoResponse, Request, Response, Result,
};

use crate::{
  errors::AppError,
  models::{Entry, PackageConfig, PackagePathname},
  utils::{encrypt::etag, swc::rewrite_javascript_esmodule},
};

pub async fn serve_module(req: &Request) -> Result<Response> {
  let entry = <&Entry>::from_request_without_body(req).await?;
  let pkg = <&PackagePathname>::from_request_without_body(req).await?;
  if entry.content_type == mime::APPLICATION_JAVASCRIPT {
    return serve_javascript_module(req).await.map_err(|_| {
      AppError::UnableGenerateModule {
        package_spec: pkg.package_spec.to_owned(),
        filename: pkg.filename.to_owned(),
      }
      .into()
    });
  }

  if entry.content_type == mime::TEXT_HTML {
    //TODO: html module
  }

  Err(AppError::InvalidContentTypeForModuleMode).map_err(Into::into)
}

async fn serve_javascript_module(req: &Request) -> poem::Result<Response> {
  let pkg_config = <&PackageConfig>::from_request_without_body(req).await?;
  let entry = <&Entry>::from_request_without_body(req).await?.to_owned();

  let code = match String::from_utf8(entry.content.to_vec()).ok() {
    Some(code) => rewrite_javascript_esmodule(code, pkg_config)?,
    None => String::default(),
  };
  //
  let resp = StatusCode::OK
    .with_header(
      header::CONTENT_TYPE,
      mime::APPLICATION_JAVASCRIPT_UTF_8.as_ref(),
    )
    .with_header(header::CACHE_CONTROL, "public, max-age=31536000")
    .with_header(header::ETAG, etag(&code)?)
    .with_header("Cache-Tag", "file, js-file, js-module")
    .with_body(code)
    .into_response();
  Ok(resp)
}
