use std::sync::Arc;
use common::ast::ASTNode;
use common::ast::module::ASTModule;
use common::ast::types::{Function, FunctionType, LiteralKind, OperatorKind, ParenthesisKind, ParenthesisState, TypeKind};
use common::errors::LangError;
use tokenizer::iterator::{Tokens, TokenSnapshot};
use tokenizer::tokens::Token;
use crate::errors::{LOAD_MODULE_ERROR, ParsingErrorHelper, UNEXPECTED_ERROR, WRONG_TYPE};
use crate::{expect_indent, expect_token};
use crate::modules::module_importer::{ModuleIdentifier, ModuleImporter, ModuleUID};
use crate::modules::module_loader::{LoadModuleResult, ModuleLoader};
use crate::parser::ParserScope;
use crate::utils::{parse_parameter_names, parse_type_error};

pub enum DeclarationKind {
    Variable(TypeKind),
    Function(Vec<String>, FunctionType),
}

pub struct Declaration {
    pub kind: DeclarationKind,
    pub body: TokenSnapshot,
}

pub struct LoadingModule {
    pub tokens: Tokens,
    pub imports: Vec<ModuleUID>,
    pub declarations: Vec<(String, Declaration)>
}

pub struct LoadingModuleLoader<'a, Importer: ModuleImporter> {
    loader: &'a mut ModuleLoader<Importer>,
}

impl<'a, Importer: ModuleImporter> LoadingModuleLoader<'a, Importer> {
    pub fn new(loader: &'a mut ModuleLoader<Importer>) -> Self {
        Self {
            loader,
        }
    }

    pub fn load(&mut self, tokens: Tokens) -> Result<LoadingModule, LangError> {
        let mut module = LoadingModule {
            tokens,
            imports: Vec::new(),
            declarations: Vec::new(),
        };

        loop {
            if !module.tokens.has_next() {
                break
            }

            let result = self.parse_declaration(&mut module);
            match result {
                Ok(DeclarationParseAction::Import(path)) => {
                    let result = self.loader.load_module(&ModuleIdentifier(path));

                    let uid = match result {
                        LoadModuleResult::Ok(uid) |
                        LoadModuleResult::AlreadyLoaded(uid) => uid,
                        LoadModuleResult::NotFound => return Err(LangError::new_parser(LOAD_MODULE_ERROR.to_string())),
                        LoadModuleResult::Err(err) => return Err(err),
                    };

                    module.imports.push(uid);
                },
                Ok(DeclarationParseAction::Declaration(name, declaration)) => {
                    module.declarations.push((name, declaration));
                },
                Err(err) => return Err(err),
            }
        }

        Ok(module)
    }

    fn parse_declaration(&mut self, module: &mut LoadingModule) -> Result<DeclarationParseAction, LangError> {
        let token = match module.tokens.pop() {
            Some(t) => t,
            None => return Err(LangError::new_parser_end_of_file()),
        };

        match token {
            Token::Import => {
                // import [path]

                // [path]
                let path = match module.tokens.pop() {
                    Some(Token::Literal(LiteralKind::String(path))) => path,
                    Some(_) => return Err(LangError::new_parser_unexpected_token()),
                    None => return Err(LangError::new_parser_end_of_file()),
                };

                // new line
                expect_token!(module.tokens.pop(), Token::NewLine);

                Ok(DeclarationParseAction::Import(path))
            },
            Token::Variable => {
                // var <name>: (type) = [value]

                // <name>
                let name = match module.tokens.pop() {
                    Some(Token::Symbol(name)) => name,
                    Some(_) => return Err(LangError::new_parser_unexpected_token()),
                    None => return Err(LangError::new_parser_end_of_file()),
                };

                // : (type)
                let type_kind = parse_type_error(&mut module.tokens)?;

                // =
                expect_token!(module.tokens.pop(), Token::Operator(OperatorKind::Assign));

                // [value]
                let body = module.tokens.snapshot();
                Self::pop_until_newline(&mut module.tokens);

                Ok(DeclarationParseAction::Declaration(
                    name,
                    Declaration {
                        kind: DeclarationKind::Variable(type_kind),
                        body,
                    },
                ))
            },
            Token::Function => {
                // func <name>((<param_name>: (type))*): (type) {body}

                // <name>
                let name = match module.tokens.pop() {
                    Some(Token::Symbol(name)) => name,
                    Some(_) => return Err(LangError::new_parser_unexpected_token()),
                    None => return Err(LangError::new_parser_end_of_file()),
                };

                // (
                expect_token!(module.tokens.pop(), Token::Parenthesis(ParenthesisKind::Round, ParenthesisState::Open));

                // (<param_name>: (type))*)
                let (param_names, param_types) = parse_parameter_names(&mut module.tokens)?;

                // : (type)
                let ret_type = parse_type_error(&mut module.tokens)?;

                expect_indent!(module.tokens);

                // {body}
                let body = module.tokens.snapshot();
                Self::pop_until_dedent(&mut module.tokens);

                let func_type = FunctionType(param_types, Box::new(ret_type));

                Ok(DeclarationParseAction::Declaration(
                    name,
                    Declaration {
                        kind: DeclarationKind::Function(param_names, func_type),
                        body,
                    }
                ))
            },
            Token::NewLine => self.parse_declaration(module),
            _ => Err(LangError::new_parser_unexpected_token()),
        }
    }

    fn pop_until_dedent(tokens: &mut Tokens) {
        let mut indentations = 0;

        loop {
            match tokens.pop() {
                Some(Token::Indent) => indentations += 1,
                Some(Token::Dedent) => {
                    if indentations == 0 {
                        break;
                    }

                    indentations -= 1;
                },
                None => break,
                Some(_) => (),
            }
        }
    }

    fn pop_until_newline(tokens: &mut Tokens) {
        loop {
            match tokens.pop() {
                Some(Token::NewLine) | None => break,
                Some(_) => (),
            }
        }
    }
}

enum DeclarationParseAction {
    Import(String),
    Declaration(String, Declaration),
}
