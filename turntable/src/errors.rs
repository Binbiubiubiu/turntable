use poem::{error::ResponseError, http::header, IntoResponse, Response};
use reqwest::StatusCode;
use thiserror::Error;

// Make our own error that wraps `anyhow::Error`.
#[derive(Debug, Error)]
pub enum AppError {
  #[error(transparent)]
  Any(#[from] anyhow::Error),
  #[error("Invalid URL: {0}")]
  InvalidURL(String),
  #[error("Invalid package name \"{package_name}\" ({reason})")]
  InvalidPackageName {
    package_name: String,
    reason: String,
  },
  #[error("Cannot find package {0}")]
  NotFoundPackage(String),
  #[error("Cannot get config for package {0}")]
  UnableGetConfigForPackage(String),
  #[error("Package {0} does not contain an ES module")]
  NotContainEsModule(String),
  #[error("Cannot find \"{filename}\" in {package_spec}")]
  NotFoundFileInPackage {
    package_spec: String,
    filename: String,
  },
  #[error("Cannot find an index in \"{filename}\" in {package_spec}")]
  NotFoundIndexFileInPackage {
    package_spec: String,
    filename: String,
  },
  #[error("module mode is available only for JavaScript and HTML files")]
  InvalidContentTypeForModuleMode,
  #[error("Cannot generate module for {package_spec}{filename}")]
  UnableGenerateModule {
    package_spec: String,
    filename: String,
  },
}

// Tell axum how to convert `AppError` into a response.
impl ResponseError for AppError {
  fn status(&self) -> StatusCode {
    match self {
      AppError::InvalidURL(_) => StatusCode::FORBIDDEN,
      AppError::InvalidPackageName { .. } => StatusCode::FORBIDDEN,
      AppError::InvalidContentTypeForModuleMode => StatusCode::FORBIDDEN,
      AppError::NotFoundPackage(_) => StatusCode::NOT_FOUND,
      AppError::NotFoundFileInPackage { .. } => StatusCode::NOT_FOUND,
      AppError::NotFoundIndexFileInPackage { .. } => StatusCode::NOT_FOUND,
      _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
  }

  fn as_response(&self) -> Response {
    let resp = Response::builder()
      .status(self.status())
      .body(self.to_string());

    match self {
      AppError::NotFoundFileInPackage { .. } => resp
        .with_header(header::CACHE_CONTROL, "public, max-age=31536000")
        .with_header("Cache-Tag", "missing, missing-entry")
        .into_response(),
      AppError::NotFoundIndexFileInPackage { .. } => resp
        .with_header(header::CACHE_CONTROL, "public, max-age=31536000")
        .with_header("Cache-Tag", "missing, missing-index")
        .into_response(),
      _ => resp,
    }
  }
}
