use std::cmp::Ordering;
use std::collections::HashSet;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use swc_common::sync::Lrc;
use swc_common::{
    comments::SingleThreadedComments,
    errors::{ColorConfig, Handler},
    SourceMap,
};

use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};
use swc_ecma_dep_graph::{analyze_dependencies};

#[derive(Debug, Eq)]
struct Module {
    id: usize,
    path_name: PathBuf,
    ast: Option<swc_ecma_ast::Module>,
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

impl Hash for Module {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.path_name.hash(hasher);
    }
}

impl PartialEq for Module {
    fn eq(&self, other: &Self) -> bool {
        self.path_name_as_str() == other.path_name_as_str()
    }
}

impl PartialOrd for Module {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

fn main() {
    let path = Path::new("./samples/entry.js").to_path_buf();
    let modules_map = &mut HashSet::new();
    traverse(path, modules_map);
}

fn traverse(path: PathBuf, modules_map: &mut HashSet<Module>) {
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

    let mut base_path = path.clone();
    base_path.pop();

    modules_map.insert(Module {
        id: modules_map.len(),
        path_name: fs::canonicalize(&path.clone()).unwrap(),
        ast: Some(module.clone()),
    });

    // I'm going to keep it simple here.
    // let dependencies = analyze_dependencies(&module.clone(),&SingleThreadedComments::default());

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

                                            if !modules_map.contains(&Module {
                                                id: 0,
                                                path_name: final_path_name.clone(),
                                                ast: None,
                                            }) {
                                                traverse(final_path_name, modules_map);
                                            }
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
        let module_map = &mut HashSet::new();
        traverse(path, module_map);
        assert_eq!(2, module_map.len());
        for (index, item) in module_map.iter().enumerate() {
            assert_eq!(index, item.id);
        }
    }
}
