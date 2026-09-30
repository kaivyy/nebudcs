//! Structured Text (ST) Compiler for IEC 61131-3.

pub mod lexer;
pub mod parser;

use crate::error::ControlError;
use crate::iec61131::ir::IecStatement;
use crate::iec61131::storage::SymbolTable;
use lexer::Lexer;
use parser::Parser;

/// Compiles Structured Text source code into variable declarations and executable IR statements.
pub fn compile_st(source: &str) -> Result<(SymbolTable, Vec<IecStatement>), ControlError> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    parser.parse_routine()
}
