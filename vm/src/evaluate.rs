use std::{ops::{Try, FromResidual, ControlFlow}, borrow::Borrow, sync::Arc, collections::HashMap};
use common::{lang_value::LangValue, types::{ReturnKind, MathOperatorKind, BoolOperatorKind}, errors::LangError, ast::{ASTNode, ASTBody}, messages::{VARIABLE_NOT_DECLARED, VARIABLE_IS_NOT_A_FUNCTION, INCORRECT_NUMBER_OF_PARAMETERS, VARIABLE_IS_NOT_A_NUMBER, INVALID_VALUE_FIELD_ACCESS}, external_functions::ExternalFunctionRunner};

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
            None => return EvalResult::Err(LangError::new_runtime($err)),
        }
    };
}


pub fn evaluate(ast: &Box<ASTNode>, scope: &Scope) -> EvalResult {
    match ast.as_ref() {
        ASTNode::Root { body } => {
            for child in body {
                evaluate(child, scope.clone())?;
            }
            
            EvalResult::Ok(LangValue::Nothing)
        },
        ASTNode::VariableDecl { name, value } => {
            let value = evaluate(value, scope.clone())?;
            scope.declare_var(name.clone(), value.clone());

            EvalResult::Ok(LangValue::Nothing)
        },
        ASTNode::VaraibleRef { name } => {
            match scope.get_var(name) {
                Some(value) => EvalResult::Ok(value.clone()),
                None => EvalResult::Err(LangError::new_runtime(VARIABLE_NOT_DECLARED.to_string())),
            }
        },
        ASTNode::VariableAsgn { name, value } => {
            let value = evaluate(value, scope.clone())?;
            scope.set_var(name, value);
            
            EvalResult::Ok(LangValue::Nothing)
        },
        ASTNode::MethodInvok { object, name, parameters } => {
            let object = evaluate(object, scope.clone())?;
            let func = match object.get_field(scope.registry.borrow(), name) {
                Some(func) => func.clone(),
                None => return EvalResult::Err(LangError::new_runtime(INVALID_VALUE_FIELD_ACCESS.to_string())),
            };
            
            let mut param_values = Vec::new();
            param_values.push(object);
            for param in parameters {
                let value = evaluate(param, scope.clone())?;
                param_values.push(value);
            }
            
            invoke_function(scope, &func, parameters, param_values)
        },
        ASTNode::FunctionInvok { variable, parameters } => {
            let func = evaluate(variable, scope.clone())?;
                    
            let mut param_values = Vec::new();
            for param in parameters {
                let value = evaluate(param, scope.clone())?;
                param_values.push(value);
            }

            invoke_function(scope, &func, parameters, param_values)
        },
        ASTNode::Literal { value } => {
            EvalResult::Ok(value.clone())
        },
        ASTNode::MathOperation { operation, left, right } => {
            let left = evaluate(left, scope.clone())?;
            let right = evaluate(right, scope.clone())?;
            
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
        ASTNode::BoolOperation { operation, left, right } => {
            let left = evaluate(left, scope.clone())?;
            let right = evaluate(right, scope.clone())?;
            
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
        ASTNode::ReturnStatement { value: Some(value ), kind } => EvalResult::Ret(evaluate(value, scope.clone())?, kind.clone()),
        ASTNode::ReturnStatement { value: None, kind } => EvalResult::Ret(LangValue::Nothing, kind.clone()),
        ASTNode::IfStatement { condition, body } => {
            let condition = evaluate(condition, scope.clone())?;
            
            if condition.truthy() {
                let if_scope = Scope::new_child(scope);

                for child in body {
                    evaluate(child, &if_scope)?;
                }
            }
            
            EvalResult::Ok(LangValue::Nothing)
        },
        ASTNode::ForStatement { left, right, body, iter_name } => {
            let left = evaluate(left, scope.clone())?.as_i32();
            let right = evaluate(right, scope.clone())?.as_i32();
            
            let min = expect_some!(left, VARIABLE_IS_NOT_A_NUMBER.to_string());
            let max = expect_some!(right, VARIABLE_IS_NOT_A_NUMBER.to_string());
            
            for i in min..max {
                let for_scope = Scope::new_child(scope.clone());
                for_scope.declare_var(iter_name.clone(), LangValue::Int(i));
                
                for child in body {
                    match evaluate(child, &for_scope) {
                        EvalResult::Ok(_) => (),
                        EvalResult::Ret(value, ReturnKind::Break) => return EvalResult::Ok(value),
                        EvalResult::Ret(value, kind) => return EvalResult::Ret(value, kind),
                        EvalResult::Err(err) => return EvalResult::Err(err),
                    }
                }
            }
            
            EvalResult::Ok(LangValue::Nothing)
        },
        ASTNode::WhileStatement { condition, body } => {
            while evaluate(condition, scope.clone())?.truthy() {
                let while_scope = Scope::new_child(scope.clone());
                
                for child in body {
                    match evaluate(child, &while_scope) {
                        EvalResult::Ok(_) => (),
                        EvalResult::Ret(value, ReturnKind::Break) => return EvalResult::Ok(value),
                        EvalResult::Ret(value, kind) => return EvalResult::Ret(value, kind),
                        EvalResult::Err(err) => return EvalResult::Err(err),
                    }
                }
            }

            EvalResult::Ok(LangValue::Nothing)
        },
        ASTNode::FieldAccess { variable, field_name } => {
            let value = evaluate(variable, scope.clone())?;
            
            let result = match value.get_field(scope.registry.borrow(), field_name) {
                Some(value) => value.clone(),
                None => LangValue::Nothing,
            };
            
            EvalResult::Ok(result)
        },
        ASTNode::VectorLiteral { values } => {
            let mut eval_values = Vec::new();
            
            for val in values {
                eval_values.push(evaluate(val, scope.clone())?);
            }
            
            EvalResult::Ok(LangValue::Vector(Arc::new(eval_values)))
        },
        ASTNode::ValueFieldAccess { variable, value } => {
            let variable = evaluate(variable, scope.clone())?;
            let value = evaluate(value, scope.clone())?;

            match variable.get_value_field(value) {
                Some(value) => EvalResult::Ok(value.clone()),
                None => EvalResult::Err(LangError::new_runtime(INVALID_VALUE_FIELD_ACCESS.to_string())),
            }
        },
        ASTNode::ObjectLiteral { values } => {
            let mut map = HashMap::new();
            
            for value in values {
                map.insert(value.0.clone(), evaluate(&value.1, scope.clone())?);
            }
            
            EvalResult::Ok(LangValue::Object(Arc::new(map)))
        },
    }
}

fn invoke_function(scope: &Scope, func: &LangValue, parameters: &ASTBody, param_values: Vec<LangValue>) -> EvalResult {
    match func {
        LangValue::Function(func) => {
            // Parameters
            if parameters.len() != func.parameters.len() {
                return EvalResult::Err(LangError::new_runtime(INCORRECT_NUMBER_OF_PARAMETERS.to_string()));
            }
    
            let func_scope = Scope::new_child(scope);
            for i in 0..parameters.len() {
                // TODO: PLS BETTER PERFORMANCE! THANKS ME OF THE FUTURE
                func_scope.declare_var(func.parameters[i].to_string(), param_values[i].clone());
            }

            for child in &func.body {
                // Matching to make the return statement stop
                match evaluate(child, &func_scope) {
                    EvalResult::Ok(_) => (),
                    EvalResult::Ret(value, ReturnKind::Return) => return EvalResult::Ok(value),
                    EvalResult::Ret(value, kind) => return EvalResult::Ret(value, kind),
                    EvalResult::Err(err) => return EvalResult::Err(err),
                }
            }
            
            EvalResult::Ok(LangValue::Nothing)
        },
        LangValue::ExtFunction(func) => {
            match (func.borrow() as &ExternalFunctionRunner).run(param_values) {
                Ok(value ) => EvalResult::Ok(value),
                Err(err) => EvalResult::Err(err),
            }
        },
        _ => return EvalResult::Err(LangError::new_runtime(VARIABLE_IS_NOT_A_FUNCTION.to_string())),
    }
}