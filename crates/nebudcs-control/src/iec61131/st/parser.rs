//! Recursive Descent Parser for IEC 61131-3 Structured Text (ST).

use crate::error::ControlError;
use crate::iec61131::ir::{IecExpr, IecStatement};
use crate::iec61131::st::lexer::Token;
use crate::iec61131::storage::{SymbolTable, VarScope, VariableDeclaration};
use crate::iec61131::types::{IecType, IecValue};

/// Parser converting a stream of Tokens into variable declarations and statements.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let tok = self.peek().clone();
        if tok != Token::Eof {
            self.pos += 1;
        }
        tok
    }

    fn check(&self, expected: &Token) -> bool {
        self.peek() == expected
    }

    fn match_token(&mut self, expected: &Token) -> bool {
        if self.check(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: &Token) -> Result<(), ControlError> {
        if self.check(expected) {
            self.advance();
            Ok(())
        } else {
            Err(ControlError::InvalidParameter(format!(
                "Expected token {:?}, found {:?}",
                expected,
                self.peek()
            )))
        }
    }

    /// Parses variable declaration blocks (`VAR ... END_VAR`) and statements.
    pub fn parse_routine(&mut self) -> Result<(SymbolTable, Vec<IecStatement>), ControlError> {
        let mut symbols = SymbolTable::new();

        // Parse optional PROGRAM header: `PROGRAM name ... END_PROGRAM`
        let mut has_program_wrapper = false;
        if self.match_token(&Token::Program) {
            has_program_wrapper = true;
            if let Token::Ident(_) = self.peek() {
                self.advance();
            }
        }

        // Parse variable blocks: VAR, VAR_INPUT, VAR_OUTPUT, etc.
        while self.is_var_block_start() {
            self.parse_var_block(&mut symbols)?;
        }

        // Parse body statements
        let mut statements = Vec::new();
        while !self.check(&Token::Eof)
            && !self.check(&Token::EndProgram)
            && !self.check(&Token::EndFunctionBlock)
        {
            statements.push(self.parse_statement()?);
        }

        if has_program_wrapper {
            let _ = self.match_token(&Token::EndProgram);
        }

        Ok((symbols, statements))
    }

    fn is_var_block_start(&self) -> bool {
        matches!(
            self.peek(),
            Token::Var
                | Token::VarInput
                | Token::VarOutput
                | Token::VarInOut
                | Token::VarGlobal
                | Token::VarConstant
                | Token::VarTemp
        )
    }

    fn parse_var_block(&mut self, symbols: &mut SymbolTable) -> Result<(), ControlError> {
        let scope = match self.advance() {
            Token::Var => VarScope::Var,
            Token::VarInput => VarScope::VarInput,
            Token::VarOutput => VarScope::VarOutput,
            Token::VarInOut => VarScope::VarInOut,
            Token::VarGlobal => VarScope::VarGlobal,
            Token::VarConstant => VarScope::VarConstant,
            Token::VarTemp => VarScope::VarTemp,
            _ => unreachable!(),
        };

        while !self.check(&Token::EndVar) && !self.check(&Token::Eof) {
            // Identifier list: e.g. `X, Y, Z : REAL := 0.0;`
            let mut var_names = Vec::new();
            match self.advance() {
                Token::Ident(name) => var_names.push(name),
                other => {
                    return Err(ControlError::InvalidParameter(format!(
                        "Expected variable name identifier, found {:?}",
                        other
                    )))
                }
            }

            while self.match_token(&Token::Comma) {
                if let Token::Ident(name) = self.advance() {
                    var_names.push(name);
                } else {
                    return Err(ControlError::InvalidParameter(
                        "Expected identifier after comma in variable declaration".into(),
                    ));
                }
            }

            // Optional physical address binding: `AT %IX0.0` or `AT %QW100`
            let mut addr_binding = None;
            if let Token::Ident(ref s) = self.peek() {
                if s.eq_ignore_ascii_case("AT") {
                    self.advance();
                    if let Token::Ident(addr) = self.advance() {
                        addr_binding = Some(addr);
                    }
                }
            }

            self.expect(&Token::Colon)?;
            let data_type = self.parse_data_type()?;

            // Optional initialization: `:= expr`
            let mut init_val = data_type.default_value();
            if self.match_token(&Token::Assign) {
                init_val = self.parse_literal_value()?;
            }

            self.expect(&Token::Semicolon)?;

            for name in var_names {
                let mut decl = VariableDeclaration::new(name, data_type.clone())
                    .with_scope(scope)
                    .with_initial_value(init_val.clone());
                if let Some(ref addr) = addr_binding {
                    decl = decl.with_address(addr.clone());
                }
                symbols.declare(decl)?;
            }
        }

        self.expect(&Token::EndVar)?;
        Ok(())
    }

    fn parse_data_type(&mut self) -> Result<IecType, ControlError> {
        let tok = self.advance();
        if let Token::Ident(name) = tok {
            match name.to_ascii_uppercase().as_str() {
                "BOOL" => Ok(IecType::Bool),
                "BYTE" => Ok(IecType::Byte),
                "WORD" => Ok(IecType::Word),
                "DWORD" => Ok(IecType::DWord),
                "LWORD" => Ok(IecType::LWord),
                "SINT" => Ok(IecType::Sint),
                "INT" => Ok(IecType::Int),
                "DINT" => Ok(IecType::Dint),
                "LINT" => Ok(IecType::Lint),
                "USINT" => Ok(IecType::Usint),
                "UINT" => Ok(IecType::Uint),
                "UDINT" => Ok(IecType::Udint),
                "ULINT" => Ok(IecType::Ulint),
                "REAL" => Ok(IecType::Real),
                "LREAL" => Ok(IecType::LReal),
                "TIME" => Ok(IecType::Time),
                "DATE" => Ok(IecType::Date),
                "TOD" => Ok(IecType::Tod),
                "DT" => Ok(IecType::Dt),
                "STRING" => Ok(IecType::String),
                "ARRAY" => {
                    self.expect(&Token::LBracket)?;
                    let low = match self.advance() {
                        Token::Int(v) => v as i32,
                        other => {
                            return Err(ControlError::InvalidParameter(format!(
                                "Expected array lower bound integer, found {:?}",
                                other
                            )))
                        }
                    };
                    self.expect(&Token::DotDot)?;
                    let high = match self.advance() {
                        Token::Int(v) => v as i32,
                        other => {
                            return Err(ControlError::InvalidParameter(format!(
                                "Expected array upper bound integer, found {:?}",
                                other
                            )))
                        }
                    };
                    self.expect(&Token::RBracket)?;
                    // Expect 'OF' keyword
                    if let Token::Of = self.peek() {
                        self.advance();
                    } else if let Token::Ident(ref s) = self.peek() {
                        if s.eq_ignore_ascii_case("OF") {
                            self.advance();
                        }
                    }
                    let elem_type = self.parse_data_type()?;
                    Ok(IecType::Array {
                        element_type: Box::new(elem_type),
                        lower_bound: low,
                        upper_bound: high,
                    })
                }
                // Named UDT or Function Block type name
                udt => Ok(IecType::Struct {
                    type_name: udt.to_string(),
                    fields: Vec::new(),
                }),
            }
        } else {
            Err(ControlError::InvalidParameter(format!(
                "Expected data type identifier, found {:?}",
                tok
            )))
        }
    }

    fn parse_literal_value(&mut self) -> Result<IecValue, ControlError> {
        match self.advance() {
            Token::True => Ok(IecValue::Bool(true)),
            Token::False => Ok(IecValue::Bool(false)),
            Token::Int(v) => Ok(IecValue::Dint(v as i32)),
            Token::Real(v) => Ok(IecValue::LReal(v)),
            Token::Time(v) => Ok(IecValue::Time(v)),
            Token::Str(s) => Ok(IecValue::String(s)),
            Token::Minus => {
                let inner = self.parse_literal_value()?;
                inner.unary_op("-")
            }
            other => Err(ControlError::InvalidParameter(format!(
                "Expected literal value for initialization, found {:?}",
                other
            ))),
        }
    }

    /// Parses an executable statement.
    pub fn parse_statement(&mut self) -> Result<IecStatement, ControlError> {
        match self.peek() {
            Token::If => self.parse_if(),
            Token::Case => self.parse_case(),
            Token::For => self.parse_for(),
            Token::While => self.parse_while(),
            Token::Repeat => self.parse_repeat(),
            Token::Return => {
                self.advance();
                self.expect(&Token::Semicolon)?;
                Ok(IecStatement::Return)
            }
            Token::Exit => {
                self.advance();
                self.expect(&Token::Semicolon)?;
                Ok(IecStatement::Exit)
            }
            Token::Ident(_) => self.parse_assignment_or_call(),
            other => Err(ControlError::InvalidParameter(format!(
                "Unexpected token at start of statement: {:?}",
                other
            ))),
        }
    }

    fn parse_assignment_or_call(&mut self) -> Result<IecStatement, ControlError> {
        let name = match self.advance() {
            Token::Ident(s) => s,
            _ => unreachable!(),
        };

        // Function block call: `INST(IN := 1, PT := T#5s);`
        if self.match_token(&Token::LParen) {
            let mut params = Vec::new();
            while !self.check(&Token::RParen) && !self.check(&Token::Eof) {
                let param_name = match self.advance() {
                    Token::Ident(p) => p,
                    other => {
                        return Err(ControlError::InvalidParameter(format!(
                            "Expected parameter name in FB call, found {:?}",
                            other
                        )))
                    }
                };
                self.expect(&Token::Assign)?;
                let expr = self.parse_expr()?;
                params.push((param_name, expr));
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
            self.expect(&Token::RParen)?;
            self.expect(&Token::Semicolon)?;
            return Ok(IecStatement::FbCall {
                instance: name.clone(),
                fb_type: "UNKNOWN".to_string(), // Resolved during execution
                params,
            });
        }

        // Array element assignment: `name[idx] := expr;`
        if self.match_token(&Token::LBracket) {
            let idx_expr = self.parse_expr()?;
            self.expect(&Token::RBracket)?;
            self.expect(&Token::Assign)?;
            let expr = self.parse_expr()?;
            self.expect(&Token::Semicolon)?;
            return Ok(IecStatement::AssignArray {
                array: name,
                index: idx_expr,
                expr,
            });
        }

        // Struct field assignment: `name.field := expr;`
        if self.match_token(&Token::Dot) {
            let field = match self.advance() {
                Token::Ident(f) => f,
                other => {
                    return Err(ControlError::InvalidParameter(format!(
                        "Expected field name after '.', found {:?}",
                        other
                    )))
                }
            };
            self.expect(&Token::Assign)?;
            let expr = self.parse_expr()?;
            self.expect(&Token::Semicolon)?;
            return Ok(IecStatement::AssignField {
                struct_var: name,
                field,
                expr,
            });
        }

        // Standard variable or output assignment: `name := expr;`
        self.expect(&Token::Assign)?;
        let expr = self.parse_expr()?;
        self.expect(&Token::Semicolon)?;

        if name.starts_with("%Q") || name.starts_with("%q") {
            Ok(IecStatement::AssignIoOutput { key: name, expr })
        } else {
            Ok(IecStatement::Assign { target: name, expr })
        }
    }

    fn parse_if(&mut self) -> Result<IecStatement, ControlError> {
        self.expect(&Token::If)?;
        let condition = self.parse_expr()?;
        self.expect(&Token::Then)?;

        let mut then_branch = Vec::new();
        while !self.check(&Token::Elsif)
            && !self.check(&Token::Else)
            && !self.check(&Token::EndIf)
            && !self.check(&Token::Eof)
        {
            then_branch.push(self.parse_statement()?);
        }

        let mut elsif_branches = Vec::new();
        while self.match_token(&Token::Elsif) {
            let cond = self.parse_expr()?;
            self.expect(&Token::Then)?;
            let mut body = Vec::new();
            while !self.check(&Token::Elsif)
                && !self.check(&Token::Else)
                && !self.check(&Token::EndIf)
                && !self.check(&Token::Eof)
            {
                body.push(self.parse_statement()?);
            }
            elsif_branches.push((cond, body));
        }

        let mut else_branch = None;
        if self.match_token(&Token::Else) {
            let mut else_body = Vec::new();
            while !self.check(&Token::EndIf) && !self.check(&Token::Eof) {
                else_body.push(self.parse_statement()?);
            }
            else_branch = Some(else_body);
        }

        self.expect(&Token::EndIf)?;
        self.expect(&Token::Semicolon)?;

        Ok(IecStatement::If {
            condition,
            then_branch,
            elsif_branches,
            else_branch,
        })
    }

    fn parse_case(&mut self) -> Result<IecStatement, ControlError> {
        self.expect(&Token::Case)?;
        let selector = self.parse_expr()?;
        self.expect(&Token::Of)?;

        let mut cases = Vec::new();
        let mut else_branch = None;

        while !self.check(&Token::EndCase) && !self.check(&Token::Eof) {
            if self.match_token(&Token::Else) {
                let mut else_body = Vec::new();
                while !self.check(&Token::EndCase) && !self.check(&Token::Eof) {
                    else_body.push(self.parse_statement()?);
                }
                else_branch = Some(else_body);
                break;
            }

            // Parse literal match values
            let mut match_vals = Vec::new();
            match_vals.push(self.parse_literal_value()?);
            while self.match_token(&Token::Comma) {
                match_vals.push(self.parse_literal_value()?);
            }

            self.expect(&Token::Colon)?;
            let mut body = Vec::new();
            while !self.check(&Token::EndCase)
                && !self.check(&Token::Else)
                && !self.is_literal_start()
                && !self.check(&Token::Eof)
            {
                body.push(self.parse_statement()?);
            }
            cases.push((match_vals, body));
        }

        self.expect(&Token::EndCase)?;
        self.expect(&Token::Semicolon)?;

        Ok(IecStatement::Case {
            selector,
            cases,
            else_branch,
        })
    }

    fn is_literal_start(&self) -> bool {
        matches!(
            self.peek(),
            Token::Int(_)
                | Token::Real(_)
                | Token::Time(_)
                | Token::Str(_)
                | Token::True
                | Token::False
        )
    }

    fn parse_for(&mut self) -> Result<IecStatement, ControlError> {
        self.expect(&Token::For)?;
        let counter = match self.advance() {
            Token::Ident(s) => s,
            other => {
                return Err(ControlError::InvalidParameter(format!(
                    "Expected counter identifier in FOR loop, found {:?}",
                    other
                )))
            }
        };

        self.expect(&Token::Assign)?;
        let from = self.parse_expr()?;
        self.expect(&Token::To)?;
        let to = self.parse_expr()?;

        let mut by = None;
        if self.match_token(&Token::By) {
            by = Some(self.parse_expr()?);
        }

        self.expect(&Token::Do)?;
        let mut body = Vec::new();
        while !self.check(&Token::EndFor) && !self.check(&Token::Eof) {
            body.push(self.parse_statement()?);
        }
        self.expect(&Token::EndFor)?;
        self.expect(&Token::Semicolon)?;

        Ok(IecStatement::For {
            counter,
            from,
            to,
            by,
            body,
        })
    }

    fn parse_while(&mut self) -> Result<IecStatement, ControlError> {
        self.expect(&Token::While)?;
        let condition = self.parse_expr()?;
        self.expect(&Token::Do)?;

        let mut body = Vec::new();
        while !self.check(&Token::EndWhile) && !self.check(&Token::Eof) {
            body.push(self.parse_statement()?);
        }
        self.expect(&Token::EndWhile)?;
        self.expect(&Token::Semicolon)?;

        Ok(IecStatement::While { condition, body })
    }

    fn parse_repeat(&mut self) -> Result<IecStatement, ControlError> {
        self.expect(&Token::Repeat)?;
        let mut body = Vec::new();
        while !self.check(&Token::Until) && !self.check(&Token::Eof) {
            body.push(self.parse_statement()?);
        }
        self.expect(&Token::Until)?;
        let until = self.parse_expr()?;
        self.expect(&Token::EndRepeat)?;
        self.expect(&Token::Semicolon)?;

        Ok(IecStatement::Repeat { body, until })
    }

    // -----------------------------------------------------------------------
    // Expression Parsing with Standard Precedence
    // -----------------------------------------------------------------------

    pub fn parse_expr(&mut self) -> Result<IecExpr, ControlError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<IecExpr, ControlError> {
        let mut left = self.parse_xor()?;
        while self.match_token(&Token::Or) {
            let right = self.parse_xor()?;
            left = IecExpr::Binary {
                op: "OR".to_string(),
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_xor(&mut self) -> Result<IecExpr, ControlError> {
        let mut left = self.parse_and()?;
        while self.match_token(&Token::Xor) {
            let right = self.parse_and()?;
            left = IecExpr::Binary {
                op: "XOR".to_string(),
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<IecExpr, ControlError> {
        let mut left = self.parse_comparison()?;
        while self.match_token(&Token::And) {
            let right = self.parse_comparison()?;
            left = IecExpr::Binary {
                op: "AND".to_string(),
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<IecExpr, ControlError> {
        let mut left = self.parse_additive()?;
        loop {
            let op = if self.match_token(&Token::Equal) {
                "="
            } else if self.match_token(&Token::NotEqual) {
                "<>"
            } else if self.match_token(&Token::Less) {
                "<"
            } else if self.match_token(&Token::LessEqual) {
                "<="
            } else if self.match_token(&Token::Greater) {
                ">"
            } else if self.match_token(&Token::GreaterEqual) {
                ">="
            } else {
                break;
            };

            let right = self.parse_additive()?;
            left = IecExpr::Binary {
                op: op.to_string(),
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<IecExpr, ControlError> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = if self.match_token(&Token::Plus) {
                "+"
            } else if self.match_token(&Token::Minus) {
                "-"
            } else {
                break;
            };

            let right = self.parse_multiplicative()?;
            left = IecExpr::Binary {
                op: op.to_string(),
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<IecExpr, ControlError> {
        let mut left = self.parse_power()?;
        loop {
            let op = if self.match_token(&Token::Star) {
                "*"
            } else if self.match_token(&Token::Slash) {
                "/"
            } else if self.match_token(&Token::Mod) {
                "MOD"
            } else {
                break;
            };

            let right = self.parse_power()?;
            left = IecExpr::Binary {
                op: op.to_string(),
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_power(&mut self) -> Result<IecExpr, ControlError> {
        let mut left = self.parse_unary()?;
        while self.match_token(&Token::Power) {
            let right = self.parse_unary()?;
            left = IecExpr::Binary {
                op: "**".to_string(),
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<IecExpr, ControlError> {
        if self.match_token(&Token::Not) {
            let expr = self.parse_unary()?;
            return Ok(IecExpr::Unary {
                op: "NOT".to_string(),
                expr: Box::new(expr),
            });
        }
        if self.match_token(&Token::Minus) {
            let expr = self.parse_unary()?;
            return Ok(IecExpr::Unary {
                op: "-".to_string(),
                expr: Box::new(expr),
            });
        }
        if self.match_token(&Token::Plus) {
            return self.parse_unary();
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<IecExpr, ControlError> {
        match self.advance() {
            Token::True => Ok(IecExpr::Literal(IecValue::Bool(true))),
            Token::False => Ok(IecExpr::Literal(IecValue::Bool(false))),
            Token::Int(v) => Ok(IecExpr::Literal(IecValue::Dint(v as i32))),
            Token::Real(v) => Ok(IecExpr::Literal(IecValue::LReal(v))),
            Token::Time(v) => Ok(IecExpr::Literal(IecValue::Time(v))),
            Token::Str(s) => Ok(IecExpr::Literal(IecValue::String(s))),
            Token::LParen => {
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(expr)
            }
            Token::Ident(name) => {
                // Check if followed by `(` -> Function call
                if self.match_token(&Token::LParen) {
                    let mut args = Vec::new();
                    while !self.check(&Token::RParen) && !self.check(&Token::Eof) {
                        args.push(self.parse_expr()?);
                        if !self.match_token(&Token::Comma) {
                            break;
                        }
                    }
                    self.expect(&Token::RParen)?;
                    return Ok(IecExpr::FunctionCall { name, args });
                }

                // Check if followed by `[` -> Array indexing
                if self.match_token(&Token::LBracket) {
                    let idx = self.parse_expr()?;
                    self.expect(&Token::RBracket)?;
                    return Ok(IecExpr::ArrayAccess {
                        array: name,
                        index: Box::new(idx),
                    });
                }

                // Check if followed by `.` -> Struct / FB field access
                if self.match_token(&Token::Dot) {
                    let field = match self.advance() {
                        Token::Ident(f) => f,
                        other => {
                            return Err(ControlError::InvalidParameter(format!(
                                "Expected field name after '.', found {:?}",
                                other
                            )))
                        }
                    };
                    return Ok(IecExpr::FieldAccess {
                        struct_var: name,
                        field,
                    });
                }

                // Input point readback or standard variable
                if name.starts_with("%I") || name.starts_with("%i") {
                    Ok(IecExpr::IoInput(name))
                } else if name.starts_with("%Q") || name.starts_with("%q") {
                    Ok(IecExpr::IoOutput(name))
                } else {
                    Ok(IecExpr::Var(name))
                }
            }
            other => Err(ControlError::InvalidParameter(format!(
                "Unexpected token in expression: {:?}",
                other
            ))),
        }
    }
}
