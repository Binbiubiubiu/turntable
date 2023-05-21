use tracing_subscriber::prelude::*;
use turntable::ListenPort;

#[inline]
fn get_port() -> u16 {
  option_env!("PORT")
    .get_or_insert("8080")
    .parse::<u16>()
    .expect("get port ok")
}

#[tokio::main]
async fn main() {
  tracing_subscriber::registry()
    .with(tracing_subscriber::fmt::layer().with_target(false))
    .with(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "turntable=debug,poem=debug".into()),
    )
    .init();

  let port: u16 = get_port();
  turntable::Server::new().listen(port).await.unwrap();
}
