use std::ops::Deref;

use poem::{FromRequest, Request, RequestBody};
use serde::Deserialize;
use serde_json::Value;

fn merge(a: &mut Value, b: &Value) {
  match (a, b) {
    (&mut Value::Object(ref mut a), Value::Object(b)) => {
      for (k, v) in b {
        merge(a.entry(k.clone()).or_insert(Value::Null), v);
      }
    }
    (a, b) => {
      *a = b.clone();
    }
  }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PackageConfig(Value);

impl PackageConfig {
  #[inline]
  pub fn get_str(&self, key: impl AsRef<str>) -> Option<&str> {
    self.get(key.as_ref()).and_then(|value| value.as_str())
  }

  #[inline]
  pub fn dependencies(&self) -> Value {
    let mut dependencies = self
      .get("dependencies")
      .map(ToOwned::to_owned)
      .unwrap_or(serde_json::json!({}));

    let empty_obj = serde_json::json!({});
    let peer_dependencies = self.get("peerDependencies").unwrap_or(&empty_obj);

    merge(&mut dependencies, peer_dependencies);
    dependencies
  }
}

impl Deref for PackageConfig {
  type Target = Value;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

#[poem::async_trait]
impl<'a> FromRequest<'a> for &'a PackageConfig {
  async fn from_request(req: &'a Request, _: &mut RequestBody) -> poem::Result<Self> {
    req
      .extensions()
      .get::<PackageConfig>()
      .ok_or(anyhow::anyhow!(
        "get package config from the request extensions"
      ))
      .map_err(Into::into)
  }
}
