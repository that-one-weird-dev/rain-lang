use core::LangError;
use std::{ops::{FromResidual, Try, ControlFlow}, sync::Arc, collections::HashMap};
use common::{ast::{ASTNode, NodeKind, types::{ReturnKind, MathOperatorKind, BoolOperatorKind}}, errors::RuntimeErrorKind};
use crate::{lang_value::LangValue, object::LangObject};
use super::scope::Scope;


pub enum EvalResult {
    Ok(LangValue),
    Ret(LangValue, ReturnKind),
    Err(LangError),
}

impl FromResidual for EvalResult {
    fn from_residual(residual: <Self as Try>::Residual) -> Self {
        residual
    }
}

impl Try for EvalResult {
    type Output = LangValue;
    type Residual = EvalResult;

    fn from_output(output: Self::Output) -> Self {
        EvalResult::Ok(output)
    }

    fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
        match self {
            EvalResult::Ok(value) => ControlFlow::Continue(value),
            EvalResult::Ret(value, kind) => ControlFlow::Break(EvalResult::Ret(value, kind)),
            EvalResult::Err(err) => ControlFlow::Break(EvalResult::Err(err)),
        }
    }
}

macro_rules! expect_some {
    ($value:expr, $err:expr) => {
        match $value {
            Some(val) => val,
            None => return EvalResult::Err(LangError::runtime($err)),
        }
    };
}

impl<'a> Scope<'a> {
    pub(crate) fn evaluate_ast(&self, ast: &ASTNode) -> EvalResult {
        match ast.kind.as_ref() {
            NodeKind::VariableDecl { name, value } => {
                let value = self.evaluate_ast(value)?;
                self.declare_var(name.clone(), value.clone());

                EvalResult::Ok(LangValue::Nothing)
            },
            NodeKind::VariableRef { module, name } => {
                match self.get_var(*module, name) {
                    Some(value) => EvalResult::Ok(value.clone()),
                    None => EvalResult::Err(LangError::runtime(RuntimeErrorKind::VarNotFound(name.clone()))),
                }
            },
            NodeKind::VariableAsgn { name, value } => {
                let value = self.evaluate_ast(value)?;
                self.set_var(name, value);
                
                EvalResult::Ok(LangValue::Nothing)
            },
            NodeKind::FunctionInvok { variable, parameters } => {
                let func = self.evaluate_ast(variable)?;

                let mut param_values = Vec::new();
                for param in parameters {
                    let value = self.evaluate_ast(param)?;
                    param_values.push(value);
                }

                self.invoke_function(&func, param_values)
            },
            NodeKind::Literal { value } => {
                EvalResult::Ok(value.clone().into())
            },
            NodeKind::MathOperation { operation, left, right } => {
                let left = self.evaluate_ast(left)?;
                let right = self.evaluate_ast(right)?;
                
                let value = match operation {
                    MathOperatorKind::Plus => left.sum(right),
                    MathOperatorKind::Minus => left.minus(right),
                    MathOperatorKind::Multiply => left.multiply(right),
                    MathOperatorKind::Divide => left.divide(right),
                    MathOperatorKind::Modulus => left.modulus(right),
                    MathOperatorKind::Power => left.power(right),
                };
                
                EvalResult::Ok(value)
            },
            NodeKind::BoolOperation { operation, left, right } => {
                let left = self.evaluate_ast(left)?;
                let right = self.evaluate_ast(right)?;
                
                let value = match operation {
                    BoolOperatorKind::Equal => left.equals(&right),
                    BoolOperatorKind::Different => left.not_equals(&right),
                    BoolOperatorKind::Bigger => left.bigger(&right),
                    BoolOperatorKind::Smaller => left.smaller(&right),
                    BoolOperatorKind::BiggerEq => left.bigger_eq(&right),
                    BoolOperatorKind::SmallerEq => left.smaller_eq(&right),
                };
                
                EvalResult::Ok(LangValue::Bool(value))
            },
            NodeKind::ReturnStatement { value: Some(value ), kind } => EvalResult::Ret(self.evaluate_ast(value)?, kind.clone()),
            NodeKind::ReturnStatement { value: None, kind } => EvalResult::Ret(LangValue::Nothing, kind.clone()),
            NodeKind::IfStatement { condition, body } => {
                let condition = self.evaluate_ast(condition)?;
                
                if condition.truthy() {
                    let if_scope = Scope::new_child(self.clone());

                    for child in body {
                        if_scope.evaluate_ast(child)?;
                    }
                }
                
                EvalResult::Ok(LangValue::Nothing)
            },
            NodeKind::ForStatement { left, right, body, iter_name } => {
                let left = self.evaluate_ast(left)?.as_i32();
                let right = self.evaluate_ast(right)?.as_i32();
                
                let min = expect_some!(left, RuntimeErrorKind::ValueNotNumber);
                let max = expect_some!(right, RuntimeErrorKind::ValueNotNumber);
                
                for i in min..max {
                    let for_scope = Scope::new_child(self.clone());
                    for_scope.declare_var(iter_name.clone(), LangValue::Int(i));
                    
                    for child in body {
                        match for_scope.evaluate_ast(child) {
                            EvalResult::Ok(_) => (),
                            EvalResult::Ret(value, ReturnKind::Break) => return EvalResult::Ok(value),
                            EvalResult::Ret(value, kind) => return EvalResult::Ret(value, kind),
                            EvalResult::Err(err) => return EvalResult::Err(err),
                        }
                    }
                }
                
                EvalResult::Ok(LangValue::Nothing)
            },
            NodeKind::WhileStatement { condition, body } => {
                while self.evaluate_ast(condition)?.truthy() {
                    let while_scope = Scope::new_child(self.clone());
                    
                    for child in body {
                        match while_scope.evaluate_ast(child) {
                            EvalResult::Ok(_) => (),
                            EvalResult::Ret(value, ReturnKind::Break) => return EvalResult::Ok(value),
                            EvalResult::Ret(value, kind) => return EvalResult::Ret(value, kind),
                            EvalResult::Err(err) => return EvalResult::Err(err),
                        }
                    }
                }

                EvalResult::Ok(LangValue::Nothing)
            },
            NodeKind::FieldAccess { variable, field_name } => {
                let value = self.evaluate_ast(variable)?;
                let result = value.get_field(field_name);
                
                EvalResult::Ok(result)
            },
            NodeKind::VectorLiteral { values } => {
                let mut eval_values = Vec::new();
                
                for val in values {
                    eval_values.push(self.evaluate_ast(val)?);
                }
                
                EvalResult::Ok(LangValue::Vector(Arc::new(eval_values)))
            },
            NodeKind::ValueFieldAccess { variable, value } => {
                let variable = self.evaluate_ast(variable)?;
                let value = self.evaluate_ast(value)?;

                EvalResult::Ok(variable.get_value_field(value))
            },
            NodeKind::ObjectLiteral { values } => {
                let mut map = HashMap::new();
                
                for value in values {
                    map.insert(value.0.clone(), self.evaluate_ast(&value.1)?);
                }
                
                EvalResult::Ok(LangValue::Object(LangObject::from_map(map)))
            },
            NodeKind::FunctionLiteral { value } => {
                EvalResult::Ok(LangValue::Function(value.clone()))
            },
        }
    }

    pub(crate) fn invoke_function(&self, func: &LangValue, param_values: Vec<LangValue>) -> EvalResult {
        match func {
            LangValue::Function(func) => {
                // Parameters
                if func.parameters.len() != param_values.len() {
                    return EvalResult::Err(LangError::runtime(RuntimeErrorKind::FuncInvalidParamCount(func.parameters.len(), param_values.len())));
                }
        
                let func_scope = Scope::new_child(self.clone());
                for i in 0..func.parameters.len() {
                    func_scope.declare_var(func.parameters[i].to_string(), param_values[i].clone());
                }

                for child in &func.body {
                    // Matching to make the return statement stop
                    match func_scope.evaluate_ast(child) {
                        EvalResult::Ok(_) => (),
                        EvalResult::Ret(value, ReturnKind::Return) => return EvalResult::Ok(value),
                        EvalResult::Ret(value, kind) => return EvalResult::Ret(value, kind),
                        EvalResult::Err(err) => return EvalResult::Err(err),
                    }
                }
                
                EvalResult::Ok(LangValue::Nothing)
            },
            LangValue::ExtFunction(func) => {
                match func.run(param_values) {
                    Ok(value ) => EvalResult::Ok(value),
                    Err(err) => EvalResult::Err(err),
                }
            },
            _ => return EvalResult::Err(LangError::runtime(RuntimeErrorKind::ValueNotFunc)),
        }
    }
}
