pub mod encrypt;
pub mod fs;
pub mod npm;
pub mod swc;
pub mod url;

use async_compression::tokio::bufread::GzipDecoder;

use mime_guess::mime;
use poem::{
  http::{header, StatusCode},
  IntoResponse,
};
use tokio::io::AsyncReadExt;
use tokio_tar::{Archive, Entry};

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

#[inline]
pub async fn read_entry_file(
  file: &mut Entry<Archive<GzipDecoder<&[u8]>>>,
) -> anyhow::Result<Vec<u8>> {
  let mut content = Vec::new();
  file.read_to_end(&mut content).await?;
  Ok(content)
}

#[inline]
pub fn strip_suffix_filename(filename: &String) -> &str {
  if filename.ends_with('/') {
    filename.strip_suffix('/').unwrap_or("/")
  } else {
    filename
  }
}
