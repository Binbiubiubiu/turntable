use std::path::PathBuf;

use mime_guess::Mime;

pub fn get_content_type(file: &PathBuf) -> Mime {
  let text_files = regex!(r"(?i)/?(\.[a-z]*rc|\.git[a-z]*|\.[a-z]*ignore|\.lock)$");
  let name = file
    .file_name()
    .and_then(|f| f.to_str())
    .expect("get file name");

  if text_files.is_match(name) {
    mime_guess::mime::TEXT_PLAIN
  } else {
    mime_guess::from_path(file).first_or(mime_guess::mime::TEXT_PLAIN)
  }
}
