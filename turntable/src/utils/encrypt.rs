use base64::Engine;
use hashes::{sha1, sha2::sha384};

pub use base64::engine::general_purpose::STANDARD as base64;

#[inline]
pub fn get_intergrity(content: impl AsRef<[u8]>) -> anyhow::Result<String> {
  let content = content.as_ref();
  let digest = sha384::hash(content);
  Ok(format!("sha384-{}", base64.encode(digest.into_bytes())))
}

#[inline]
pub fn etag(content: impl AsRef<[u8]>) -> anyhow::Result<String> {
  let content = content.as_ref();
  let len = &format!("{:#x}", content.len())[2..];
  let digest = sha1::hash(content);
  let hash = &base64.encode(digest.into_bytes())[..27];
  Ok(format!("W/\"{len}-{hash}\""))
}
