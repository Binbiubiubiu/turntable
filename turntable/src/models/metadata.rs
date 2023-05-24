use std::path::PathBuf;

use chrono::{DateTime, Utc};
use mime_guess::Mime;
use serde::{Serialize, Serializer};

const FORMAT: &str = "%a, %d %b %Y %H:%M:%S GMT";

fn mime_serialize<S>(mime: &Mime, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  serializer.serialize_str(mime.as_ref())
}

fn last_modified_serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  let s = format!("{}", date.format(FORMAT));
  serializer.serialize_str(&s)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Metadata {
  #[serde(rename_all = "camelCase")]
  Directory { path: PathBuf, files: Vec<Metadata> },
  #[serde(rename_all = "camelCase")]
  File {
    path: PathBuf,
    #[serde(serialize_with = "mime_serialize")]
    content_type: Mime,
    integrity: String,
    #[serde(serialize_with = "last_modified_serialize")]
    last_modified: DateTime<Utc>,
    size: u64,
  },
}

impl Metadata {
  pub fn new_dir(path: impl AsRef<str>) -> Self {
    Metadata::Directory {
      path: PathBuf::from(path.as_ref()),
      files: vec![],
    }
  }
}

#[cfg(test)]
mod tests {
  use chrono::NaiveDateTime;

  use super::*;

  #[test]
  fn test_dir_serialize() {
    let dir = Metadata::Directory {
      path: PathBuf::from("/dir"),
      files: vec![],
    };

    assert_eq!(
      serde_json::to_string(&dir).unwrap(),
      r#"{"type":"directory","path":"/dir","files":[]}"#
    );
  }

  #[test]
  fn test_file_serialize() {
    let file = Metadata::File {
      path: PathBuf::from("/file"),
      content_type: mime_guess::mime::APPLICATION_JAVASCRIPT_UTF_8,
      integrity: "integrity".into(),
      last_modified: DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp_opt(1684947507, 748).unwrap_or_default(),
        Utc,
      ),
      size: 10086,
    };

    assert_eq!(
      serde_json::to_string(&file).unwrap(),
      r#"{"type":"file","path":"/file","contentType":"application/javascript; charset=utf-8","integrity":"integrity","lastModified":"Wed, 24 May 2023 16:58:27 GMT","size":10086}"#
    );
  }
}
