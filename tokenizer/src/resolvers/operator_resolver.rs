use common::{errors::TokenizerErrorKind, ast::types::{OperatorKind, BoolOperatorKind, MathOperatorKind}, tokens::TokenKind};
use super::resolver::{Resolver, AddResult};

pub struct OperatorResolver {
    chars: String,
}

impl OperatorResolver {
    pub fn new() -> Self {
        Self {
            chars: String::new(),
        }
    }
    
    fn end_operator(&self) -> Result<TokenKind, TokenizerErrorKind> {
        Ok(match self.chars.as_str() {
            // Operators
            "=" => TokenKind::Operator(OperatorKind::Assign),
            ".." => TokenKind::Operator(OperatorKind::Range),
            "," => TokenKind::Operator(OperatorKind::Comma),
            "." => TokenKind::Operator(OperatorKind::Dot),
            ":" => TokenKind::Operator(OperatorKind::Colon),
            "@" => TokenKind::Operator(OperatorKind::At),
            
            // Math operator
            "+" => TokenKind::MathOperator(MathOperatorKind::Plus),
            "-" => TokenKind::MathOperator(MathOperatorKind::Minus),
            "*" => TokenKind::MathOperator(MathOperatorKind::Multiply),
            "/" => TokenKind::MathOperator(MathOperatorKind::Divide),
            "%" => TokenKind::MathOperator(MathOperatorKind::Modulus),
            "^" => TokenKind::MathOperator(MathOperatorKind::Power),
            
            // Bool opreator
            "==" => TokenKind::BoolOperator(BoolOperatorKind::Equal),
            "!=" => TokenKind::BoolOperator(BoolOperatorKind::Different),
            ">" => TokenKind::BoolOperator(BoolOperatorKind::Bigger),
            "<" => TokenKind::BoolOperator(BoolOperatorKind::Smaller),
            ">=" => TokenKind::BoolOperator(BoolOperatorKind::BiggerEq),
            "<=" => TokenKind::BoolOperator(BoolOperatorKind::SmallerEq),
            
            // Fallback
            _ => return Err(TokenizerErrorKind::InvalidOperatorToken),
        })
    }
}

impl Resolver for OperatorResolver {
    fn add(&mut self, char: char) -> AddResult {
        match char {
            '=' | '.' | ',' | '!' | '>' | '<' | '+' | '-' | '*' | '/' | '%' | '^' | ':' | '@' => {
                self.chars.push(char);
                AddResult::Ok
            },

            _ => {
                match self.end_operator() {
                    Ok(token) => AddResult::Change(token, char),
                    Err(err) => AddResult::Err(err),
                }
            },
        }
    }
}