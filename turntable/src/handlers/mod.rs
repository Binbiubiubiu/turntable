mod file;
mod meta;
mod module;

use poem::{Request, Response};

use crate::{
  extractors::PackageQuery,
  handlers::{file::serve_file, meta::serve_directory_metadata, module::serve_module},
};

#[poem::handler]
pub async fn handle_pkg_pathname(query: PackageQuery, req: &Request) -> poem::Result<Response> {
  if query.meta.is_some() {
    return serve_directory_metadata(req).await;
  }

  if query.module.is_some() {
    return serve_module(req).await;
  }

  serve_file(req).await
}
