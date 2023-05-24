use std::path::PathBuf;

use async_compression::tokio::bufread::GzipDecoder;
use bytes::Bytes;

use poem::{web::Json, FromRequest, IntoResponse, Request, Response, Result};

use tokio_stream::StreamExt;
use tokio_tar::Archive;

use crate::{
  errors::AppError,
  models::{Metadata, Mtime, PackagePathname},
  utils::{
    encrypt::get_intergrity, fs::get_content_type, npm::get_package, read_entry_file,
    strip_suffix_filename,
  },
};

pub async fn find_entry(
  steam: Bytes,
  filename: impl AsRef<str>,
) -> anyhow::Result<Option<Metadata>> {
  // filename = /some/dir/name
  let filename = filename.as_ref();
  let mut found_entry = None;

  let tar = GzipDecoder::new(&*steam);
  let mut ar = Archive::new(tar);

  let mut entries = ar.entries()?;
  while let Some(Ok(mut file)) = entries.next().await {
    // Make sure there wasn't an I/O error
    let mut path = file.path()?.to_path_buf();

    if !path.starts_with("/") {
      path = PathBuf::from("/").join(path.iter().skip(1).collect::<PathBuf>());
    }

    if !file.header().entry_type().is_file() || path.to_str().unwrap_or_default() != filename {
      continue;
    }

    let content = read_entry_file(&mut file).await?;

    let entry = Metadata::File {
      content_type: get_content_type(&path),
      path,
      last_modified: Mtime::from(file.header().mtime()?).try_into()?,
      size: file.header().size()?,
      integrity: get_intergrity(&content)?,
    };

    found_entry = Some(entry);
  }

  Ok(found_entry)
}

pub async fn serve_file_metadata(req: &Request) -> Result<Response> {
  let pkg = <&PackagePathname>::from_request_without_body(req).await?;
  let stream = get_package(&pkg.package_name, &pkg.package_version).await?;

  let filename = strip_suffix_filename(&pkg.filename);
  let entry = find_entry(stream, filename).await?;

  match entry {
    Some(entry) => Ok(Json(entry).into_response()),
    None => Err(AppError::NotFoundFileInPackage {
      package_spec: pkg.package_spec.clone(),
      filename: pkg.filename.clone(),
    })
    .map_err(Into::into),
  }
}
