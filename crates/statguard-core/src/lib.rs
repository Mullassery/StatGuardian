pub mod ast;
pub mod compiler;
pub mod error;
pub mod parser;
pub mod streaming;

pub use ast::DataContract;
pub use compiler::Compiler;
pub use error::{CoreError, CoreResult};
pub use parser::parse;
pub use streaming::{StreamConfig, StreamingWindow, TumblingWindowExecutor, WindowType};

/// Parse DSL text and compile to an execution DAG in one step.
pub fn parse_and_compile(
    dsl: &str,
) -> CoreResult<Vec<(DataContract, compiler::dag::ExecutionDag)>> {
    let contracts = parser::parse(dsl)?;
    let compiler = Compiler::new();
    contracts
        .into_iter()
        .map(|c| {
            let dag = compiler.compile(&c)?;
            Ok((c, dag))
        })
        .collect()
}
