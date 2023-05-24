use poem::{Endpoint, FromRequest, Middleware, Request, Result};

use crate::{errors::AppError, models::PackagePathname};

#[inline]
fn is_hash(value: impl AsRef<str>) -> bool {
  let value = value.as_ref();
  value.len() == 32 && regex!("^[a-f0-9]+$").is_match(value)
}

pub struct ValidatePackageName;

impl<E: Endpoint> Middleware<E> for ValidatePackageName {
  type Output = ValidatePackageNameEndpoint<E>;

  fn transform(&self, ep: E) -> Self::Output {
    ValidatePackageNameEndpoint { ep }
  }
}

pub struct ValidatePackageNameEndpoint<E> {
  ep: E,
}

#[poem::async_trait]
impl<E: Endpoint> Endpoint for ValidatePackageNameEndpoint<E> {
  type Output = E::Output;

  async fn call(&self, req: Request) -> Result<Self::Output> {
    let package_name = <&PackagePathname>::from_request_without_body(&req)
      .await
      .map(|pkg| pkg.package_name.to_owned())?;

    if is_hash(&package_name) {
      return Err(AppError::InvalidPackageName {
        package_name,
        reason: "cannot be a hash".into(),
      })
      .map_err(Into::into);
    }
    validate_npm_package_name::validate(&package_name).map_err(|e| {
      AppError::InvalidPackageName {
        package_name,
        reason: e.to_owned(),
      }
    })?;

    self.ep.call(req).await
  }
}
