use std::sync::Arc;

use once_cell::sync::Lazy;
use swc::{self, try_with_handler};
use swc_common::{comments::SingleThreadedComments, errors::ColorConfig, SourceMap, GLOBALS};
use swc_core::ecma::{transforms::base::pass::noop, visit::as_folder};

use crate::extractors::{Entry, PackageConfig};

static ORIGIN: Lazy<&'static str> =
  Lazy::new(|| option_env!("ORIGIN").unwrap_or("https://unpkg.com"));

pub fn rewrite_javascript_esmodule(
  entry: &Entry,
  package_config: &PackageConfig,
) -> anyhow::Result<String> {
  let cm = Arc::<SourceMap>::default();

  let compiler = swc::Compiler::new(cm.clone());

  GLOBALS
    .set(&Default::default(), || {
      try_with_handler(
        cm.clone(),
        swc::HandlerOpts {
          color: ColorConfig::Auto,
          skip_filename: false,
        },
        |handler| {
          //   let fm = cm
          //     .load_file(Path::new("turntable/examples/transform-input.js"))
          //     .expect("failed to load file");

          let fm = cm.new_source_file(
            swc_common::FileName::Anon,
            String::from_utf8(entry.content.to_vec())?,
          );

          compiler.process_js_with_custom_pass(
            fm,
            None,
            handler,
            &Default::default(),
            SingleThreadedComments::default(),
            |_| {
              as_folder(path_url_rewrite::TransformVisitor::new(
                *ORIGIN,
                package_config.dependencies(),
              ))
            },
            |_| noop(),
          )
        },
      )
    })
    .map(|r| r.code)
}
