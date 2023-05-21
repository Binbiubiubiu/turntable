#[macro_use]
pub(crate) mod macros;

use serde::Deserialize;
use serde_json::Value;
use swc_core::ecma::{
  ast::*,
  transforms::testing::test,
  visit::{as_folder, FoldWith, VisitMut, VisitMutWith},
};
use swc_core::plugin::{plugin_transform, proxies::TransformPluginProgramMetadata};

fn is_absolute_url(value: impl AsRef<str>) -> bool {
  let value = value.as_ref();
  let is_valid_url = url::Url::parse(value).is_ok();
  let is_probably_url_without_protocol = value.starts_with("//");
  is_valid_url || is_probably_url_without_protocol
}

fn is_bare_identifier(value: impl AsRef<str>) -> bool {
  let value = value.as_ref();
  !value.starts_with('.') && !value.starts_with('/')
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransformVisitor {
  origin: String,
  dependencies: Value,
}

impl TransformVisitor {
  pub fn new(origin: impl Into<String>, dependencies: Value) -> Self {
    Self {
      origin: origin.into(),
      dependencies,
    }
  }

  pub fn rewrite_value(&mut self, s: &mut Str) {
    if is_absolute_url(&s.value) {
      return;
    }

    let value = if is_bare_identifier(&s.value) {
      let Some(matches) = regex!(r"^((?:@[^/]+/)?[^/]+)(/.*)?$").captures(&s.value)else {
        return;
      };

      let Some(package_name) = matches.get(1).map(|s|s.as_str()) else {
        return;
      };

      let version = self
        .dependencies
        .get(package_name)
        .and_then(|s| s.as_str())
        .unwrap_or("latest");
      let file = matches.get(2).map(|s| s.as_str()).unwrap_or_default();

      format!("{}/{package_name}@{version}{file}?module", self.origin)
    } else {
      format!("{}?module", s.value)
    };

    println!("value: {value}");

    *s = Str {
      span: Default::default(),
      value: value.into(),
      raw: None,
    };
  }
}

impl VisitMut for TransformVisitor {
  // Implement necessary visit_mut_* methods for actual custom transform.
  // A comprehensive list of possible visitor methods can be found here:
  // https://rustdoc.swc.rs/swc_ecma_visit/trait.VisitMut.html
  fn visit_mut_call_expr(&mut self, n: &mut swc_core::ecma::ast::CallExpr) {
    n.visit_mut_children_with(self);

    if !n.callee.is_import() {
      return;
    }

    let path = n.args.get_mut(0).map(|s| s.expr.as_mut());

    if let Some(Expr::Lit(Lit::Str(s))) = path {
      self.rewrite_value(s);
    }
  }

  fn visit_mut_export_all(&mut self, n: &mut swc_core::ecma::ast::ExportAll) {
    n.visit_mut_children_with(self);

    let s = n.src.as_mut();
    self.rewrite_value(s);

    println!("visit_mut_export_all");
  }

  fn visit_mut_named_export(&mut self, n: &mut swc_core::ecma::ast::NamedExport) {
    n.visit_mut_children_with(self);

    if let Some(src) = n.src.as_mut() {
      let s = src.as_mut();
      self.rewrite_value(s);
    }

    println!(
      "visit_mut_named_export: {}",
      n.src.clone().unwrap().value.to_string()
    );
  }

  fn visit_mut_import_decl(&mut self, n: &mut swc_core::ecma::ast::ImportDecl) {
    n.visit_mut_children_with(self);

    let s = n.src.as_mut();
    self.rewrite_value(s);

    println!("visit_mut_import_decl: {}", n.src.value.to_string());
  }
}

/// An example plugin function with macro support.
/// `plugin_transform` macro interop pointers into deserialized structs, as well
/// as returning ptr back to host.
///
/// It is possible to opt out from macro by writing transform fn manually
/// if plugin need to handle low-level ptr directly via
/// `__transform_plugin_process_impl(
///     ast_ptr: *const u8, ast_ptr_len: i32,
///     unresolved_mark: u32, should_enable_comments_proxy: i32) ->
///     i32 /*  0 for success, fail otherwise.
///             Note this is only for internal pointer interop result,
///             not actual transform result */`
///
/// This requires manual handling of serialization / deserialization from ptrs.
/// Refer swc_plugin_macro to see how does it work internally.
#[plugin_transform]
pub fn process_transform(program: Program, metadata: TransformPluginProgramMetadata) -> Program {
  let visitor = serde_json::from_str::<TransformVisitor>(
    &metadata
      .get_transform_plugin_config()
      .expect("failed to get plugin config for path-url-rewrite"),
  )
  .expect("invalid config for styled-path-url-rewrite");

  program.fold_with(&mut as_folder(visitor))
}

// An example to test plugin transform.
// Recommended strategy to test plugin's transform is verify
// the Visitor's behavior, instead of trying to run `process_transform` with mocks
// unless explicitly required to do so.

#[cfg(test)]
static MOCK_ORIGIN: &str = "https://www.test.com";

test!(
  Default::default(),
  |_| as_folder(TransformVisitor::new(
    MOCK_ORIGIN,
    serde_json::json!({
      "turntable":"1.0.1"
    })
  )),
  test_call_expr,
  // Input codes
  r#"import("turntable").then()"#,
  // Output codes after transformed with plugin
  &format!(r#"import("{MOCK_ORIGIN}/turntable@1.0.1?module").then()"#)
);

test!(
  Default::default(),
  |_| as_folder(TransformVisitor::new(MOCK_ORIGIN, serde_json::json!({}))),
  test_export_all,
  // Input codes
  r#"export * from "turntable""#,
  // Output codes after transformed with plugin
  &format!(r#"export * from "{MOCK_ORIGIN}/turntable@latest?module""#)
);

test!(
  Default::default(),
  |_| as_folder(TransformVisitor::new(
    MOCK_ORIGIN,
    serde_json::json!({
      "turntable":"1.0.1"
    })
  )),
  test_named_export,
  // Input codes
  r#"export { name as a } from "turntable/index.js""#,
  // Output codes after transformed with plugin
  &format!(r#"export {{ name as a }} from "{MOCK_ORIGIN}/turntable@1.0.1/index.js?module""#)
);

test!(
  Default::default(),
  |_| as_folder(TransformVisitor::new(
    MOCK_ORIGIN,
    serde_json::json!({
      "turntable":"1.0"
    })
  )),
  test_import_decl,
  // Input codes
  r#"import { name as a } from "./index.js""#,
  // Output codes after transformed with plugin
  r#"import { name as a } from "./index.js?module""#
);
