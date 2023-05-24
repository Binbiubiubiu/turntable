use std::{collections::HashMap, ops::Deref, path::PathBuf};

use crate::{
  errors::AppError,
  models::{Entry, PackagePathname, PackageQuery},
  utils::{
    encrypt::get_intergrity,
    fs::get_content_type,
    npm::{get_package, SearchEntry},
    redirect,
    url::create_pkg_url,
  },
};
use async_compression::tokio::bufread::GzipDecoder;
use bytes::Bytes;
use chrono::NaiveDateTime;
use poem::{
  http::header, Endpoint, FromRequest, IntoResponse, Middleware, Request, Response, Result,
};
use tokio::io::AsyncReadExt;
use tokio_stream::StreamExt;
use tokio_tar::{Archive, EntryType};

#[inline]
fn file_redirect(pkg: &PackagePathname, entry: &Entry, raw_query: Option<&str>) -> Response {
  redirect(create_pkg_url(
    &pkg.package_name,
    &pkg.package_version,
    entry.path.to_string_lossy(),
    raw_query,
  ))
  .with_header(header::CACHE_CONTROL, "public, max-age=31536000")
  .with_header("Cache-Tag", "redirect, file-redirect")
  .into_response()
}

#[inline]
fn index_redirect(pkg: &PackagePathname, entry: &Entry, raw_query: Option<&str>) -> Response {
  redirect(create_pkg_url(
    &pkg.package_name,
    &pkg.package_version,
    entry.path.to_string_lossy(),
    raw_query,
  ))
  .with_header(header::CACHE_CONTROL, "public, max-age=31536000")
  .with_header("Cache-Tag", "redirect, index-redirect")
  .into_response()
}

pub async fn search_entries(
  steam: Bytes,
  filename: impl AsRef<str>,
) -> anyhow::Result<SearchEntry> {
  let filename = filename.as_ref();
  let tar = GzipDecoder::new(steam.deref());
  let mut ar = Archive::new(tar);

  let js_entry_filename = format!("{filename}.js");
  let json_entry_filename = format!("{filename}.json");

  let mut matching_entries = HashMap::new();
  let mut found_entry: Option<Entry> = None;

  if filename == "/" {
    let entry = Entry {
      path: PathBuf::from(filename),
      entry_type: EntryType::Directory,
      ..Default::default()
    };
    found_entry = Some(entry.clone());
    matching_entries.insert(filename.into(), entry);
  }

  let mut entries = ar.entries()?;
  while let Some(Ok(mut file)) = entries.next().await {
    // Make sure there wasn't an I/O error
    let mut path = file.path()?.to_path_buf();

    if !path.starts_with("/") {
      path = PathBuf::from("/").join(path.iter().skip(1).collect::<PathBuf>());
    }

    let mut entry = Entry {
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

    if !entry.entry_type.is_file() || !entry.path.starts_with(filename) {
      continue;
    }

    for p in entry.path.ancestors() {
      let dir = p.parent().and_then(|d| d.to_str());
      if dir.is_none() || dir.is_some_and(|f| f == "/") {
        break;
      }

      let dir = dir.unwrap();

      if !matching_entries.contains_key(dir) {
        matching_entries.insert(
          dir.into(),
          Entry {
            path: PathBuf::from(dir),
            entry_type: EntryType::Directory,
            ..Default::default()
          },
        );
      }
    }

    let path = entry.path.to_str().expect("get entry path");
    if path == filename || path == js_entry_filename || path == json_entry_filename {
      if let Some(ref f_entry) = found_entry {
        let f_path = f_entry.path.to_str().expect("get found_entry path");
        if f_path != filename
          && (path == filename || (path == js_entry_filename && f_path == json_entry_filename))
        {
          read_entry_file(&mut file, &mut entry).await?;

          found_entry = Some(entry.clone());
        }
      } else {
        read_entry_file(&mut file, &mut entry).await?;

        found_entry = Some(entry.clone());
      }
    }

    matching_entries.insert(entry.path.display().to_string(), entry);
  }

  Ok(SearchEntry {
    found_entry,
    matching_entries,
  })
}

async fn read_entry_file(
  file: &mut tokio_tar::Entry<Archive<GzipDecoder<&[u8]>>>,
  entry: &mut Entry,
) -> anyhow::Result<()> {
  let mut content = Vec::new();
  file.read_to_end(&mut content).await?;
  entry.integrity = get_intergrity(&content)?;
  entry.content = content.into();
  Ok(())
}

pub struct FindEntry;

impl<E: Endpoint> Middleware<E> for FindEntry {
  type Output = FindEntryEndpoint<E>;

  fn transform(&self, ep: E) -> Self::Output {
    FindEntryEndpoint { ep }
  }
}

pub struct FindEntryEndpoint<E> {
  ep: E,
}

#[poem::async_trait]
impl<E: Endpoint> Endpoint for FindEntryEndpoint<E> {
  type Output = Response;

  async fn call(&self, mut req: Request) -> Result<Self::Output> {
    if PackageQuery::from_request_without_body(&req)
      .await?
      .meta
      .is_some()
    {
      return Ok(self.ep.call(req).await?.into_response());
    }

    let pkg = <&PackagePathname>::from_request_without_body(&req).await?;

    let stream = get_package(&pkg.package_name, &pkg.package_version).await?;
    let SearchEntry {
      found_entry: entry,
      matching_entries,
    } = search_entries(stream, &pkg.filename).await?;

    let entry = match entry {
      Some(entry) if entry.entry_type.is_file() && entry.path.to_string_lossy() != pkg.filename => {
        return Ok(file_redirect(pkg, &entry, req.uri().query()));
      }
      Some(entry) if entry.entry_type.is_dir() => {
        let index_entry = matching_entries
          .get(&format!("{}/index.js", pkg.filename))
          .or(matching_entries.get(&format!("{}/index.json", pkg.filename)));

        return match index_entry {
          Some(entry) if entry.entry_type.is_file() => {
            Ok(index_redirect(pkg, entry, req.uri().query()))
          }
          _ => Err(AppError::NotFoundIndexFileInPackage {
            filename: pkg.filename.clone(),
            package_spec: pkg.package_spec.clone(),
          })
          .map_err(Into::into),
        };
      }
      None => {
        return Err(AppError::NotFoundFileInPackage {
          filename: pkg.filename.clone(),
          package_spec: pkg.package_spec.clone(),
        })
        .map_err(Into::into);
      }
      Some(entry) => entry,
    };

    req.extensions_mut().insert(entry);

    Ok(self.ep.call(req).await?.into_response())
  }
}
