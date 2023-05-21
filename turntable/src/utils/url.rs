pub fn create_pkg_url(
  package_name: impl AsRef<str>,
  package_version: impl AsRef<str>,
  filename: impl AsRef<str>,
  query: Option<&str>,
) -> String {
  let package_name = package_name.as_ref();
  let package_version = package_version.as_ref();
  let filename = filename.as_ref();

  let mut url = format!("/{package_name}");
  if !package_version.is_empty() {
    url += format!("@{package_version}").as_str();
  }

  if !filename.is_empty() {
    url += filename;
  }

  url + query.unwrap_or_default()
}
