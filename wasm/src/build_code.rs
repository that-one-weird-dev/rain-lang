use wasm_encoder::{BlockType, Function, Instruction};
use common::ast::{ASTBody, ASTNode, NodeKind};
use common::ast::types::{BoolOperatorKind, LiteralKind, MathOperatorKind};
use common::errors::LangError;
use crate::errors::{FUNC_NOT_FOUND, LOCAL_NOT_FOUND, UNSUPPORTED_FUNC_INVOKE};

pub struct ModuleBuilder {
    functions: Vec<String>,
}

impl ModuleBuilder {
    pub fn new(functions: Vec<String>) -> Self {
        Self {
            functions,
        }
    }

    fn get_func(&self, name: &String) -> Result<u32, LangError> {
        let func = self.functions
            .iter()
            .position(|l| l == name);

        match func {
            Some(func) => Ok(func as u32),
            None => Err(LangError::new_runtime(FUNC_NOT_FOUND.to_string())),
        }
    }
}

pub struct FunctionBuilder<'a> {
    module_builder: &'a mut ModuleBuilder,
    func: &'a mut Function,
    locals: Vec<String>,
}

impl<'a> FunctionBuilder<'a> {
    pub fn new(module_builder: &'a mut ModuleBuilder, func: &'a mut Function, locals: Vec<String>) -> Self {
        Self {
            module_builder,
            func,
            locals,
        }
    }

    pub fn end_build(self) {
        self.func.instruction(&Instruction::End);
    }

    pub fn build_statement(&mut self, node: &ASTNode) -> Result<(), LangError> {
        match node.kind.as_ref() {
            NodeKind::VariableDecl { name, value } => {
                self.locals.push(name.clone());
                let id = self.locals.len() as u32 - 1;

                self.build_statement(value)?;

                self.func.instruction(&Instruction::LocalSet(id));
            },
            NodeKind::VariableRef { module: _, name } => {
                let local = self.get_local(name)?;

                self.func.instruction(&Instruction::LocalGet(local));
            },
            NodeKind::VariableAsgn { name, value } => {
                self.build_statement(value)?;

                let local = self.get_local(name)?;

                self.func.instruction(&Instruction::LocalSet(local));
            },
            NodeKind::FunctionInvok { variable, parameters } => {
                // TODO: Support for other kinds of invocations
                let name = match variable.kind.as_ref() {
                    NodeKind::VariableRef { name, module: _ } => name,
                    _ => return Err(LangError::new_runtime(UNSUPPORTED_FUNC_INVOKE.to_string())),
                };

                let func_id = self.module_builder.get_func(name)?;

                for param in parameters {
                    self.build_statement(param)?;
                }

                self.func.instruction(&Instruction::Call(func_id));
            },
            NodeKind::Literal { value } => {
                match value {
                    LiteralKind::Nothing => (),
                    LiteralKind::Int(i) => {
                        self.func.instruction(&Instruction::I32Const(*i));
                    },
                    LiteralKind::Float(f) => {
                        self.func.instruction(&Instruction::F32Const(*f));
                    },
                    LiteralKind::String(_) => todo!(),
                };
            },
            NodeKind::MathOperation { operation, left, right } => {
                self.build_statement(left)?;
                self.build_statement(right)?;

                let op = match operation {
                    MathOperatorKind::Plus => Instruction::I32Add,
                    MathOperatorKind::Minus => Instruction::I32Sub,
                    MathOperatorKind::Multiply => Instruction::I32Mul,
                    MathOperatorKind::Divide => Instruction::I32DivS,
                    MathOperatorKind::Modulus => todo!(),
                    MathOperatorKind::Power => todo!(),
                };

                self.func.instruction(&op);
            },
            NodeKind::BoolOperation { operation, left, right } => {
                self.build_statement(left)?;
                self.build_statement(right)?;

                let op = match operation {
                    BoolOperatorKind::Equal => Instruction::I32Eq,
                    BoolOperatorKind::Different => Instruction::I32Ne,
                    BoolOperatorKind::Bigger => Instruction::I32GtS,
                    BoolOperatorKind::Smaller => Instruction::I32LtS,
                    BoolOperatorKind::BiggerEq => Instruction::I32GeS,
                    BoolOperatorKind::SmallerEq => Instruction::I32LeS,
                };

                self.func.instruction(&op);
            },
            NodeKind::ReturnStatement { kind: _ , value } => {
                match value {
                    Some(value) => {
                        self.build_statement(value)?;
                    }
                    None => ()
                }

                self.func.instruction(&Instruction::Return);
            },
            NodeKind::IfStatement { condition, body } => {
                self.build_statement(condition)?;

                self.func.instruction(&Instruction::If(BlockType::Empty));

                for node in body {
                    self.build_statement(node)?;
                }

                self.func.instruction(&Instruction::End);
            },
            NodeKind::ForStatement { .. } => {}
            NodeKind::WhileStatement { .. } => {}
            NodeKind::FieldAccess { .. } => {}
            NodeKind::VectorLiteral { .. } => {}
            NodeKind::ObjectLiteral { .. } => {}
            NodeKind::FunctionLiteral { .. } => {}
            NodeKind::ValueFieldAccess { .. } => {}
        }

        Ok(())
    }

    pub fn get_local_count(body: &ASTBody) -> usize {
        let mut res = 0;

        for node in body {
            Self::get_local_count_node(node, &mut res);
        }

        res
    }

    fn get_local_count_node(node: &ASTNode, res: &mut usize) {
        match node.kind.as_ref() {
            NodeKind::VariableDecl { .. } => *res += 1,
            NodeKind::ForStatement { body, .. } => {
                *res += 1;

                for node in body {
                    Self::get_local_count_node(node, res);
                }
            }
            NodeKind::IfStatement { body, .. } => {
                for node in body {
                    Self::get_local_count_node(node, res);
                }
            }
            NodeKind::WhileStatement { body, .. } => {
                for node in body {
                    Self::get_local_count_node(node, res);
                }
            }

            NodeKind::Literal { .. } => {}
            NodeKind::FunctionInvok { .. } => {}
            NodeKind::MathOperation { .. } => {}
            NodeKind::BoolOperation { .. } => {}
            NodeKind::ReturnStatement { .. } => {}
            NodeKind::FieldAccess { .. } => {}
            NodeKind::VectorLiteral { .. } => {}
            NodeKind::ObjectLiteral { .. } => {}
            NodeKind::FunctionLiteral { .. } => {}
            NodeKind::ValueFieldAccess { .. } => {}
            NodeKind::VariableAsgn { .. } => {}
            NodeKind::VariableRef { .. } => {}
        }
    }

    fn get_local(&self, name: &String) -> Result<u32, LangError> {
        let local = self.locals
            .iter()
            .position(|l| l == name);

        match local {
            Some(local) => Ok(local as u32),
            None => Err(LangError::new_runtime(LOCAL_NOT_FOUND.to_string())),
        }
    }
}