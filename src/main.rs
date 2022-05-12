use std::env;
use std::path::Path;

use swc_common::sync::Lrc;
use swc_common::{
    errors::{ColorConfig, Handler},
    FileName, SourceMap,
};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};

fn main() {
    let cm: Lrc<SourceMap> = Default::default();
    let path = env::current_dir()?;
    let handler =
    Handler::with_tty_emitter(ColorConfig::Auto, true, false,
        Some(cm.clone()));

    // Real usage
    let fm = cm
        .load_file(Path::new("./samples/entry.js"))
        .expect("failed to load entry.js");
    // let fm = cm.new_source_file(
    //     FileName::Custom("test.js".into()),
    //     "function foo() {}".into(),
    // );
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

    let _module = parser
        .parse_module()
        .map_err(|mut e| {
            // Unrecoverable fatal error occurred
            e.into_diagnostic(&handler).emit()
        })
        .expect("failed to parser module");
        
    println!("_module:{:?}",_module.body);
}