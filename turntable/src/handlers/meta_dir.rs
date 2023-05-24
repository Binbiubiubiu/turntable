use std::{collections::HashMap, path::PathBuf};

use async_compression::tokio::bufread::GzipDecoder;
use bytes::Bytes;

use poem::{web::Json, FromRequest, IntoResponse, Request, Response, Result};

use tokio_stream::StreamExt;
use tokio_tar::Archive;

use crate::{
  models::{Metadata, Mtime, PackagePathname},
  utils::{
    encrypt::get_intergrity, fs::get_content_type, npm::get_package, read_entry_file,
    strip_suffix_filename,
  },
};

pub async fn find_matching_entries(
  steam: Bytes,
  filename: impl AsRef<str>,
) -> anyhow::Result<HashMap<PathBuf, Metadata>> {
  // filename = /some/dir/name
  let filename = filename.as_ref();
  let mut matching_entries = HashMap::new();
  let file_path = PathBuf::from(filename);
  matching_entries.insert(file_path.clone(), Metadata::new_dir(filename));

  let tar = GzipDecoder::new(&*steam);
  let mut ar = Archive::new(tar);

  let mut entries = ar.entries()?;
  while let Some(Ok(mut file)) = entries.next().await {
    // Make sure there wasn't an I/O error
    let mut path = file.path()?.to_path_buf();

    if !path.starts_with("/") {
      path = PathBuf::from("/").join(path.iter().skip(1).collect::<PathBuf>());
    }

    for p in path.ancestors() {
      let dir = match p.parent().and_then(|d| d.to_str()) {
        Some(dir) if dir != "/" => dir,
        _ => break,
      };

      if dir.starts_with(filename) {
        matching_entries.insert(
          dir.into(),
          Metadata::Directory {
            path: PathBuf::from(dir),
            files: vec![],
          },
        );
      }
    }

    if !file.header().entry_type().is_file() || !path.starts_with(filename) {
      continue;
    }

    let content = read_entry_file(&mut file).await?;

    matching_entries.insert(
      path.clone(),
      Metadata::File {
        content_type: get_content_type(&path),
        path,
        integrity: get_intergrity(&content)?,
        last_modified: Mtime::from(file.header().mtime()?).try_into()?,
        size: file.header().size()?,
      },
    );
  }

  Ok(matching_entries)
}

fn get_metadata(mut entry: Metadata, entries: &HashMap<PathBuf, Metadata>) -> Metadata {
  if let Metadata::Directory { files, path, .. } = &mut entry {
    *files = entries
      .iter()
      .filter(|&(key, _)| path != key && key.parent().is_some_and(|key| key == path))
      .map(|(_, e)| get_metadata(e.clone(), entries))
      .collect();
  }
  entry
}

pub async fn serve_directory_metadata(req: &Request) -> Result<Response> {
  let pkg = <&PackagePathname>::from_request_without_body(req).await?;
  let stream = get_package(&pkg.package_name, &pkg.package_version).await?;

  let filename = strip_suffix_filename(&pkg.filename);
  let entries = find_matching_entries(stream, filename).await?;
  let metadata = entries
    .get(&PathBuf::from(filename))
    .map(|entry| get_metadata(entry.clone(), &entries));

  let resp = match metadata {
    Some(entry) => Json(entry).into_response(),
    None => ().into_response(),
  };
  Ok(resp)
}
