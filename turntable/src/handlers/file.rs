use poem::{
  http::{header, StatusCode},
  FromRequest, IntoResponse, Request, Response,
};

use crate::{
  models::Entry,
  utils::{encrypt::etag, get_content_type_header},
};

pub async fn serve_file(req: &Request) -> poem::Result<Response> {
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
    .with_header(header::LAST_MODIFIED, entry.last_modified)
    .with_header(header::ETAG, etag(&entry.content)?)
    .with_header("Cache-Tag", tags.join(", "))
    .with_body(entry.content)
    .into_response();

  Ok(resp)
}
