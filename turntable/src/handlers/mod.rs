mod file;
mod meta_dir;
mod meta_file;
mod module;

use poem::{Request, Response};

use crate::{
  handlers::{
    file::serve_file, meta_dir::serve_directory_metadata, meta_file::serve_file_metadata,
    module::serve_module,
  },
  models::PackageQuery,
};

#[poem::handler]
pub async fn handle_pkg_pathname(query: PackageQuery, req: &Request) -> poem::Result<Response> {
  if query.meta.is_some() {
    return match req.uri().path().ends_with('/') {
      true => serve_directory_metadata(req).await,
      false => serve_file_metadata(req).await,
    };
  }

  if query.module.is_some() {
    return serve_module(req).await;
  }

  serve_file(req).await
}
