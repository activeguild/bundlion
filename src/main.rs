use std::collections::HashSet;
use std::path::{Path, PathBuf};

use swc_common::sync::Lrc;
use swc_common::{
    errors::{ColorConfig, Handler},
    SourceMap,
};

use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};

#[derive(Debug, Eq, Hash, PartialEq)]
struct Module {
    id: usize,
    path_name: PathBuf,
    ast: swc_ecma_ast::Module,
}

impl Module {
    fn path_name_as_str(&self) -> String {
        self.path_name
            .clone()
            .into_os_string()
            .into_string()
            .unwrap()
    }
}

fn main() {
    let path = Path::new("./samples/entry.js").to_path_buf();
    let modules_map = traverse(path, &HashSet::new());

    println!("modules_map:{:?}", modules_map);
}

fn traverse(path: PathBuf, parent_modules_map: &HashSet<Module>) -> HashSet<Module> {
    let mut modules_map: HashSet<Module> = HashSet::new();
    let cm: Lrc<SourceMap> = Default::default();
    let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

    let fm = cm.load_file(path.as_path()).expect(&format!(
        "failed to load {:?}",
        path.as_path().to_str().unwrap()
    ));

    let lexer = Lexer::new(
        // We want to parse ecmascript
        Syntax::Es(Default::default()),
        // EsVersion defaults to es5
        Default::default(),
        StringInput::from(&*fm),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    for e in parser.take_errors() {
        e.into_diagnostic(&handler).emit();
    }
    let module = parser
        .parse_module()
        .map_err(|mut e| {
            // Unrecoverable fatal error occurred
            e.into_diagnostic(&handler).emit()
        })
        .expect("failed to parser module");

    modules_map.insert(Module {
        id: modules_map.len(),
        path_name: path.clone(),
        ast: module.clone(),
    });

    let mut base_path = path.clone();
    base_path.pop();

    for module_item in module.body {
        match module_item {
            swc_ecma_ast::ModuleItem::Stmt(stmt) => match stmt {
                swc_ecma_ast::Stmt::Decl(decl) => match decl {
                    swc_ecma_ast::Decl::Var(var) => {
                        for decl in var.decls {
                            if let Some(init) = decl.init {
                                match *init {
                                    swc_ecma_ast::Expr::Call(call) => {
                                        let is_require = match call.callee {
                                            swc_ecma_ast::Callee::Expr(expr) => match *expr {
                                                swc_ecma_ast::Expr::Ident(ident) => {
                                                    ident.sym.to_string() == "require"
                                                }
                                                _ => false,
                                            },
                                            _ => false,
                                        };

                                        if is_require && call.args.len() == 1 {
                                            let args = call.args[0].clone();
                                            let module_name = match *args.expr {
                                                swc_ecma_ast::Expr::Lit(lit) => match lit {
                                                    swc_ecma_ast::Lit::Str(str) => {
                                                        str.value.to_string()
                                                    }
                                                    _ => "".to_string(),
                                                },
                                                _ => "".to_string(),
                                            };
                                            let final_path_name =
                                                get_path_name(base_path.clone(), module_name);

                                            modules_map.extend(traverse(
                                                final_path_name,
                                                parent_modules_map,
                                            ));
                                        }
                                    }
                                    _ => (),
                                }
                            }
                        }
                    }
                    _ => (),
                },
                _ => (),
            },
            _ => (),
        }
    }

    modules_map
}

fn get_path_name(mut base_path: PathBuf, file_name: String) -> PathBuf {
    if !is_node_module(&file_name) {
        base_path.push(get_file_name(file_name));
        return base_path.canonicalize().unwrap();
    }

    base_path
}

fn get_file_name(mut file_name: String) -> String {
    if file_name.starts_with(".") {
        file_name.remove(0);
    }
    if file_name.starts_with("/") {
        file_name.remove(0);
    }
    if !file_name.ends_with(".js") {
        file_name.push_str(".js");
    }

    file_name
}

fn is_node_module(file_name: &str) -> bool {
    !file_name.starts_with(".")
}

#[cfg(test)]
mod tests {
    use crate::traverse;
    use std::{collections::HashSet, path::Path};

    #[test]
    fn main_01() {
        let path = Path::new("./samples/entry.js").to_path_buf();
        let module_map = traverse(path, &HashSet::new());
        println!("module_map:{:?}", module_map);
    }
}
