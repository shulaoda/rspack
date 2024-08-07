// https://github.com/webpack/webpack/blob/main/lib/WarnCaseSensitiveModulesPlugin.js

use std::collections::HashMap;

use rspack_core::{
  ApplyContext, Compilation, CompilationSeal, CompilerOptions, Logger, Module, ModuleGraph, Plugin,
  PluginContext,
};
use rspack_error::{Diagnostic, Result};
use rspack_hook::{plugin, plugin_hook};

#[plugin]
#[derive(Debug, Default)]
pub struct WarnCaseSensitiveModulesPlugin;

impl WarnCaseSensitiveModulesPlugin {
  pub fn create_sensitive_modules_warning(
    &self,
    modules: &Vec<&dyn Module>,
    graph: &ModuleGraph,
  ) -> String {
    let mut message =
      String::from("There are multiple modules with names that only differ in casing.\n");

    for m in modules {
      if let Some(boxed_m) = graph.module_by_identifier(&m.identifier()) {
        let mut module_msg = format!("  - {}\n", m.identifier());
        graph
          .get_incoming_connections(&boxed_m.identifier())
          .iter()
          .for_each(|c| {
            if let Some(original_identifier) = c.original_module_identifier {
              module_msg.push_str(&format!("    - used by {}\n", original_identifier));
            }
          });

        message.push_str(&module_msg);
      }
    }

    message
  }
}

#[plugin_hook(CompilationSeal for WarnCaseSensitiveModulesPlugin)]
async fn seal(&self, compilation: &mut Compilation) -> Result<()> {
  let logger = compilation.get_logger(self.name());
  let start = logger.time("check case sensitive modules");
  let mut diagnostics: Vec<Diagnostic> = vec![];
  let module_graph = compilation.get_module_graph();
  let mut module_without_case_map: HashMap<String, HashMap<String, &Box<dyn Module>>> =
    HashMap::new();

  for module in module_graph.modules().values() {
    // Ignore `data:` URLs, because it's not a real path
    if let Some(normal_module) = module.as_normal_module() {
      if normal_module
        .resource_resolved_data()
        .encoded_content
        .is_some()
      {
        continue;
      }
    }

    let identifier = module.identifier().to_string();
    let lower_identifier = identifier.to_lowercase();
    let lower_map = module_without_case_map.entry(lower_identifier).or_default();
    lower_map.insert(identifier, module);
  }

  // sort by module identifier, guarantee the warning order
  let mut case_map_vec = module_without_case_map.into_iter().collect::<Vec<_>>();
  case_map_vec.sort_by(|a, b| a.0.cmp(&b.0));

  for (_, lower_map) in case_map_vec {
    if lower_map.values().len() > 1 {
      let mut case_modules = lower_map.values().map(|m| m.as_ref()).collect::<Vec<_>>();
      case_modules.sort_by_key(|m| m.identifier());
      diagnostics.push(Diagnostic::warn(
        "Sensitive Modules Warn".to_string(),
        self.create_sensitive_modules_warning(&case_modules, &compilation.get_module_graph()),
      ));
    }
  }

  compilation.extend_diagnostics(diagnostics);
  logger.time_end(start);
  Ok(())
}

// This Plugin warns when there are case sensitive modules in the compilation
// which can cause unexpected behavior when deployed on a case-insensitive environment
// it is executed in hook `compilation.seal`
impl Plugin for WarnCaseSensitiveModulesPlugin {
  fn name(&self) -> &'static str {
    "rspack.WarnCaseSensitiveModulesPlugin"
  }

  fn apply(
    &self,
    ctx: PluginContext<&mut ApplyContext>,
    _options: &mut CompilerOptions,
  ) -> Result<()> {
    ctx.context.compilation_hooks.seal.tap(seal::new(self));
    Ok(())
  }
}
