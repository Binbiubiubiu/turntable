use poem::{Endpoint, Middleware, Request, Result};

use crate::models::PackagePathname;

pub struct ValidatePackagePathname;

impl<E: Endpoint> Middleware<E> for ValidatePackagePathname {
  type Output = ValidatePackagePathnameEndpoint<E>;

  fn transform(&self, ep: E) -> Self::Output {
    ValidatePackagePathnameEndpoint { ep }
  }
}

pub struct ValidatePackagePathnameEndpoint<E> {
  ep: E,
}

#[poem::async_trait]
impl<E: Endpoint> Endpoint for ValidatePackagePathnameEndpoint<E> {
  type Output = E::Output;

  async fn call(&self, mut req: Request) -> Result<Self::Output> {
    let parsed = req.uri().path().parse::<PackagePathname>()?;
    req.extensions_mut().insert(parsed);

    self.ep.call(req).await
  }
}
