#[macro_use]
pub(crate) mod macros;

mod errors;
mod handlers;
mod middlewares;
mod models;
mod utils;

use std::time::Duration;

use middlewares::{
  FindEntry, ValidateFilename, ValidatePackageName, ValidatePackagePathname, ValidatePackageVersion,
};
use poem::{
  endpoint::{BoxEndpoint, StaticFileEndpoint},
  get,
  http::header,
  listener::TcpListener,
  middleware::{Compression, Cors, SetHeader, Tracing},
  web::CompressionLevel,
  EndpointExt, Route,
};
use tokio::signal;

#[poem::async_trait]
pub trait ListenPort {
  async fn listen(self, port: u16) -> anyhow::Result<()>;
}

pub struct Server {
  ep: BoxEndpoint<'static>,
}

impl Server {
  pub fn new() -> Self {
    Self::default()
  }
}

impl Default for Server {
  fn default() -> Self {
    let ep = get(handlers::handle_pkg_pathname)
      .with(FindEntry)
      .with(ValidateFilename)
      .with(ValidatePackageVersion)
      .with(ValidatePackageName)
      .with(ValidatePackagePathname);

    let ep = Route::new()
      .at("/*pkg", ep)
      .at(
        "/favicon.ico",
        StaticFileEndpoint::new("./assets/favicon.ico")
          .with(SetHeader::new().overriding(header::CACHE_CONTROL, "public, max-age=31536000")),
      )
      .at(
        "/robots.txt",
        StaticFileEndpoint::new("./assets/robots.txt")
          .with(SetHeader::new().overriding(header::CACHE_CONTROL, "public, max-age=31536000")),
      )
      .with(Tracing)
      .with(Compression::new().with_quality(CompressionLevel::Fastest))
      .with(Cors::new());

    Self { ep: ep.boxed() }
  }
}

#[poem::async_trait]
impl ListenPort for Server {
  async fn listen(self, port: u16) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{port}");
    let server = poem::Server::new(TcpListener::bind(&addr));
    server
      .run_with_graceful_shutdown(self.ep, shutdown_signal(), Some(Duration::from_secs(5)))
      .await?;
    Ok(())
  }
}

async fn shutdown_signal() {
  let ctrl_c = async {
    signal::ctrl_c()
      .await
      .expect("failed to install Ctrl+C handler")
  };

  #[cfg(unix)]
  let terminate = async {
    signal::unix::signal(signal::unix::SignalKind::terminate())
      .expect("failed to install signal handler")
      .recv()
      .await;
  };

  #[cfg(not(unix))]
  let terminate = std::future::pending::<()>();

  tokio::select! {
      _ = ctrl_c => {},
      _ = terminate => {},
  }

  tracing::info!("signal received, starting graceful shutdown");
}
