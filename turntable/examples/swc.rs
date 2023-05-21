use std::sync::Arc;

use swc::{self, try_with_handler};
use swc_common::{comments::SingleThreadedComments, errors::ColorConfig, SourceMap, GLOBALS};
use swc_core::ecma::{transforms::base::pass::noop, visit::as_folder};
fn main() {
  let cm = Arc::<SourceMap>::default();

  let c = swc::Compiler::new(cm.clone());

  // swc_plugin_runner::

  let output = GLOBALS
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
            "import a  from 'turntable/index.js'".into(),
          );

          c.process_js_with_custom_pass(
            fm,
            None,
            handler,
            &Default::default(),
            SingleThreadedComments::default(),
            |_| {
              as_folder(path_url_rewrite::TransformVisitor::new(
                "https://www.test.com",
                serde_json::json!({
                  "turntable":"1.0.1"
                }),
              ))
            },
            |_| noop(),
          )
        },
      )
    })
    .unwrap();

  println!("{}", output.code);
}
