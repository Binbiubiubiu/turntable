use std::{collections::HashMap, path::PathBuf};

use async_compression::tokio::bufread::GzipDecoder;
use bytes::Bytes;
use chrono::NaiveDateTime;
use mime_guess::Mime;
use poem::{web::Json, FromRequest, IntoResponse, Request, Response, Result};
use serde::{Serialize, Serializer};
use tokio::io::AsyncReadExt;
use tokio_stream::StreamExt;
use tokio_tar::{Archive, EntryType};

use crate::{
  extractors::PackagePathname,
  utils::{encrypt::get_intergrity, fs::get_content_type, npm::get_package},
};

fn mime_serialize<S>(x: &Mime, s: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  s.serialize_str(x.as_ref())
}

fn entry_type_serialize<S>(x: &EntryType, s: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  let ty = match x {
    EntryType::Regular => "file",
    EntryType::Directory => "directory",
    _ => "other",
  };
  s.serialize_str(ty)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
  pub path: PathBuf,
  #[serde(serialize_with = "entry_type_serialize", rename = "type")]
  pub entry_type: EntryType,
  pub files: Vec<Metadata>,
  #[serde(serialize_with = "mime_serialize")]
  pub content_type: Mime,
  pub integrity: String,
  pub last_modified: String,
  pub size: u64,
}

impl Default for Metadata {
  fn default() -> Self {
    Self {
      path: Default::default(),
      entry_type: EntryType::Regular,
      content_type: mime_guess::mime::TEXT_PLAIN,
      integrity: Default::default(),
      last_modified: Default::default(),
      size: Default::default(),
      files: Default::default(),
    }
  }
}

pub async fn find_matching_entries(
  steam: Bytes,
  filename: impl AsRef<str>,
) -> anyhow::Result<HashMap<PathBuf, Metadata>> {
  // filename = /some/dir/name
  let filename = filename.as_ref();
  let mut matching_entries = HashMap::new();
  let file_path = PathBuf::from(filename);
  matching_entries.insert(
    file_path.clone(),
    Metadata {
      path: file_path,
      entry_type: EntryType::Directory,
      ..Default::default()
    },
  );

  let tar = GzipDecoder::new(&*steam);
  let mut ar = Archive::new(tar);

  let mut entries = ar.entries()?;
  while let Some(file) = entries.next().await {
    // Make sure there wasn't an I/O error
    let mut file = file?;
    let mut path = file.path()?.to_path_buf();

    if !path.starts_with("/") {
      path = PathBuf::from("/").join(path.iter().skip(1).collect::<PathBuf>());
    }

    let mut entry = Metadata {
      content_type: get_content_type(&path),
      path,
      entry_type: file.header().entry_type(),
      last_modified: NaiveDateTime::from_timestamp_opt(file.header().mtime()?.try_into()?, 0)
        .expect("get last_modified")
        .format("%a, %d %b %Y %H:%M:%S GMT")
        .to_string(),
      size: file.header().size()?,
      ..Default::default()
    };

    for p in entry.path.ancestors() {
      let dir = match p.parent().and_then(|d| d.to_str()) {
        Some(dir) if dir != "/" => dir,
        _ => break,
      };

      if dir.starts_with(filename) {
        matching_entries.insert(
          dir.into(),
          Metadata {
            path: PathBuf::from(dir),
            entry_type: EntryType::Directory,
            ..Default::default()
          },
        );
      }
    }

    if entry.entry_type != EntryType::Regular || !entry.path.starts_with(filename) {
      continue;
    }

    read_entry_file(&mut file, &mut entry).await?;

    matching_entries.insert(entry.path.clone(), entry);
  }

  Ok(matching_entries)
}

async fn read_entry_file(
  file: &mut tokio_tar::Entry<Archive<GzipDecoder<&[u8]>>>,
  entry: &mut Metadata,
) -> anyhow::Result<()> {
  let mut content = Vec::new();
  file.read_to_end(&mut content).await?;
  entry.integrity = get_intergrity(&content)?;
  Ok(())
}

fn get_metadata(entry: &Metadata, entries: &HashMap<PathBuf, Metadata>) -> Metadata {
  let mut metadata = Metadata {
    path: entry.path.to_owned(),
    entry_type: entry.entry_type,
    ..Default::default()
  };
  match entry.entry_type {
    EntryType::Regular => {
      metadata.content_type = entry.content_type.clone();
      metadata.integrity = entry.integrity.clone();
      metadata.last_modified = entry.last_modified.clone();
      metadata.size = entry.size;
    }
    EntryType::Directory => {
      metadata.files = entries
        .iter()
        .filter(|(key, _)| entry.path != **key && key.parent().is_some_and(|key| key == entry.path))
        .map(|(key, _)| entries.get(key).unwrap())
        .map(|e| get_metadata(e, entries))
        .collect();
    }
    _ => {}
  }
  metadata
}

pub async fn serve_directory_metadata(req: &Request) -> Result<Response> {
  let pkg = <&PackagePathname>::from_request_without_body(req).await?;
  let stream = get_package(&pkg.package_name, &pkg.package_version).await?;

  let filename = pkg.filename.strip_suffix('/').unwrap_or("/");
  let entries = find_matching_entries(stream, filename).await?;
  let metadata = entries
    .get(&PathBuf::from(filename))
    .map(|entry| get_metadata(entry, &entries));

  let resp = match metadata {
    Some(entry) => Json(entry).into_response(),
    None => ().into_response(),
  };
  Ok(resp.into_response())
}
