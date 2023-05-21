use std::path::PathBuf;

use bytes::Bytes;
use chrono::NaiveDateTime;
use mime_guess::mime::Mime;
use poem::{FromRequest, Request, RequestBody};
use tokio_tar::EntryType;

#[derive(Debug, Clone)]
pub struct Entry {
  pub path: PathBuf,
  pub entry_type: EntryType,
  pub content_type: Mime,
  pub integrity: String,
  pub last_modified: NaiveDateTime,
  pub size: u64,
  pub content: Bytes,
}

impl Default for Entry {
  fn default() -> Self {
    Self {
      path: Default::default(),
      entry_type: EntryType::Regular,
      content_type: mime_guess::mime::TEXT_PLAIN,
      integrity: Default::default(),
      last_modified: Default::default(),
      size: Default::default(),
      content: Default::default(),
    }
  }
}

#[poem::async_trait]
impl<'a> FromRequest<'a> for &'a Entry {
  async fn from_request(req: &'a Request, _: &mut RequestBody) -> poem::Result<Self> {
    req
      .extensions()
      .get::<Entry>()
      .ok_or(anyhow::anyhow!("get entry from the request extensions"))
      .map_err(Into::into)
  }
}
