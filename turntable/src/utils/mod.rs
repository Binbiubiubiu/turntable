pub mod encrypt;
pub mod fs;
pub mod npm;
pub mod swc;
pub mod url;

use mime_guess::mime;
use poem::{
  http::{header, StatusCode},
  IntoResponse,
};

#[inline]
pub fn redirect(path: impl AsRef<str>) -> impl IntoResponse {
  StatusCode::FOUND.with_header(header::LOCATION, path.as_ref())
}

pub fn get_content_type_header(ty: impl AsRef<str>) -> String {
  let ty = ty.as_ref();
  if ty == mime::APPLICATION_JAVASCRIPT {
    mime::APPLICATION_JAVASCRIPT_UTF_8.to_string()
  } else {
    ty.to_owned()
  }
}
