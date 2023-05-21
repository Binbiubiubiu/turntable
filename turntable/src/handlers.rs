use mime_guess::mime;
use poem::{
  http::{header, StatusCode},
  FromRequest, IntoResponse, Request, Response,
};

use crate::{
  errors::AppError,
  extractors::{Entry, PackageConfig, PackagePathname, PackageQuery},
  utils::{encrypt::etag, get_content_type_header, swc::rewrite_javascript_esmodule},
};

#[poem::handler]
pub async fn handle_pkg_pathname(
  query: PackageQuery,
  entry: &Entry,
  req: &Request,
) -> poem::Result<Response> {
  let _entry = entry.to_owned();

  if query.meta.is_some() {
    return serve_module(req).await;
  }

  if query.module.is_some() {
    return serve_module(req).await;
  }

  serve_file(req).await
}

async fn serve_module(req: &Request) -> poem::Result<Response> {
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
  let pkg_config: &PackageConfig = <&PackageConfig>::from_request_without_body(req).await?;
  let entry = <&Entry>::from_request_without_body(req).await?;

  let code = rewrite_javascript_esmodule(entry, pkg_config)?;
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

async fn serve_file(req: &Request) -> poem::Result<Response> {
  let entry = <&Entry>::from_request_without_body(req).await?.to_owned();

  let mut tags = vec!["file"];
  if let Some(ext) = entry.path.extension().and_then(|s| s.to_str()) {
    tags.push(ext);
  }

  let resp = StatusCode::OK
    .with_header(
      header::CONTENT_TYPE,
      get_content_type_header(entry.content_type),
    )
    .with_header(header::CONTENT_LENGTH, format!("{}", entry.size))
    .with_header(header::CACHE_CONTROL, "public, max-age=31536000")
    .with_header(
      header::LAST_MODIFIED,
      entry
        .last_modified
        .format("%a, %d %b %Y %H:%M:%S GMT")
        .to_string(),
    )
    .with_header(header::ETAG, etag(&entry.content)?)
    .with_header("Cache-Tag", tags.join(", "))
    .with_body(entry.content)
    .into_response();

  Ok(resp)
}
