use crate::error::{Error, Result};
use crate::function::pdf_function::Bounds;
use crate::syntax::lexer::{Lexer, Token};

const MAX_STACK: usize = 100;

#[derive(Clone, Debug, PartialEq)]
enum Instruction {
    Number(f64),
    Bool(bool),
    Operator(String),
    If(Vec<Instruction>),
    IfElse(Vec<Instruction>, Vec<Instruction>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Value {
    Number(f64),
    Bool(bool),
}

impl Value {
    fn number(self) -> f64 {
        match self {
            Value::Number(value) => value,
            Value::Bool(value) => f64::from(u8::from(value)),
        }
    }

    fn truth(self) -> bool {
        match self {
            Value::Bool(value) => value,
            Value::Number(value) => value != 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PostScriptFunction {
    pub bounds: Bounds,
    program: Vec<Instruction>,
}

impl PostScriptFunction {
    pub fn read(bounds: Bounds, source: &[u8]) -> Result<Self> {
        let mut lexer = Lexer::new(source, 0);
        match lexer.next_token()? {
            Some(Token::Keyword(b"{")) => {}
            _ => return Err(Error::new("PostScript function does not start with {")),
        }
        let program = block(&mut lexer)?;
        Ok(Self { bounds, program })
    }

    pub fn evaluate(&self, inputs: &[f64]) -> Vec<f64> {
        let mut stack: Vec<Value> = inputs.iter().map(|&value| Value::Number(value)).collect();
        run(&self.program, &mut stack);
        let outputs = self.bounds.range.as_ref().map_or(stack.len(), Vec::len);
        let start = stack.len().saturating_sub(outputs);
        let mut values: Vec<f64> = stack[start..].iter().map(|value| value.number()).collect();
        values.resize(outputs, 0.0);
        values
    }
}

fn block(lexer: &mut Lexer) -> Result<Vec<Instruction>> {
    let mut instructions = Vec::new();
    let mut pending: Vec<Vec<Instruction>> = Vec::new();
    loop {
        let token = lexer.next_token()?.ok_or_else(|| Error::new("PostScript function ends inside a block"))?;
        match token {
            Token::Int(value) => instructions.push(Instruction::Number(value as f64)),
            Token::Real(value) => instructions.push(Instruction::Number(value)),
            Token::Keyword(b"{") => pending.push(block(lexer)?),
            Token::Keyword(b"}") => return Ok(instructions),
            Token::Keyword(b"true") => instructions.push(Instruction::Bool(true)),
            Token::Keyword(b"false") => instructions.push(Instruction::Bool(false)),
            Token::Keyword(b"if") => {
                let body = pending.pop().ok_or_else(|| Error::new("if without a block"))?;
                instructions.push(Instruction::If(body));
            }
            Token::Keyword(b"ifelse") => {
                let otherwise = pending.pop().ok_or_else(|| Error::new("ifelse without blocks"))?;
                let then = pending.pop().ok_or_else(|| Error::new("ifelse without blocks"))?;
                instructions.push(Instruction::IfElse(then, otherwise));
            }
            Token::Keyword(word) => {
                instructions.push(Instruction::Operator(String::from_utf8_lossy(word).into_owned()))
            }
            _ => return Err(Error::new("unexpected token in PostScript function")),
        }
    }
}

fn run(program: &[Instruction], stack: &mut Vec<Value>) {
    for instruction in program {
        if stack.len() > MAX_STACK {
            return;
        }
        match instruction {
            Instruction::Number(value) => stack.push(Value::Number(*value)),
            Instruction::Bool(value) => stack.push(Value::Bool(*value)),
            Instruction::If(body) => {
                if stack.pop().is_some_and(Value::truth) {
                    run(body, stack);
                }
            }
            Instruction::IfElse(then, otherwise) => {
                let condition = stack.pop().is_some_and(Value::truth);
                run(if condition { then } else { otherwise }, stack);
            }
            Instruction::Operator(operator) => apply(operator, stack),
        }
    }
}

fn apply(operator: &str, stack: &mut Vec<Value>) {
    let mut pop = || stack.pop().map_or(0.0, Value::number);
    let unary = |stack: &mut Vec<Value>, f: fn(f64) -> f64| {
        let value = stack.pop().map_or(0.0, Value::number);
        stack.push(Value::Number(f(value)));
    };
    match operator {
        "add" | "sub" | "mul" | "div" | "idiv" | "mod" | "atan" | "exp" | "bitshift" => {
            let right = pop();
            let left = pop();
            let result = match operator {
                "add" => left + right,
                "sub" => left - right,
                "mul" => left * right,
                "div" => {
                    if right == 0.0 {
                        0.0
                    } else {
                        left / right
                    }
                }
                "idiv" => {
                    if right as i64 == 0 {
                        0.0
                    } else {
                        (left as i64 / right as i64) as f64
                    }
                }
                "mod" => {
                    if right as i64 == 0 {
                        0.0
                    } else {
                        (left as i64 % right as i64) as f64
                    }
                }
                "atan" => left.atan2(right).to_degrees().rem_euclid(360.0),
                "exp" => left.powf(right),
                _ => {
                    let shift = right as i64;
                    if shift >= 0 {
                        ((left as i64) << shift.min(63)) as f64
                    } else {
                        ((left as i64) >> (-shift).min(63)) as f64
                    }
                }
            };
            stack.push(Value::Number(result));
        }
        "neg" => unary(stack, |value| -value),
        "abs" => unary(stack, f64::abs),
        "ceiling" => unary(stack, f64::ceil),
        "floor" => unary(stack, f64::floor),
        "round" => unary(stack, |value| (value + 0.5).floor()),
        "truncate" | "cvi" => unary(stack, f64::trunc),
        "cvr" => unary(stack, |value| value),
        "sqrt" => unary(stack, |value| value.max(0.0).sqrt()),
        "sin" => unary(stack, |value| value.to_radians().sin()),
        "cos" => unary(stack, |value| value.to_radians().cos()),
        "ln" => unary(stack, |value| if value > 0.0 { value.ln() } else { 0.0 }),
        "log" => unary(stack, |value| if value > 0.0 { value.log10() } else { 0.0 }),
        "eq" | "ne" | "gt" | "ge" | "lt" | "le" => {
            let right = stack.pop();
            let left = stack.pop();
            let (left, right) = (left.map_or(0.0, Value::number), right.map_or(0.0, Value::number));
            let truth = match operator {
                "eq" => left == right,
                "ne" => left != right,
                "gt" => left > right,
                "ge" => left >= right,
                "lt" => left < right,
                _ => left <= right,
            };
            stack.push(Value::Bool(truth));
        }
        "and" | "or" | "xor" => {
            let right = stack.pop().unwrap_or(Value::Bool(false));
            let left = stack.pop().unwrap_or(Value::Bool(false));
            let value = match (left, right) {
                (Value::Bool(a), Value::Bool(b)) => Value::Bool(match operator {
                    "and" => a && b,
                    "or" => a || b,
                    _ => a ^ b,
                }),
                (a, b) => {
                    let (a, b) = (a.number() as i64, b.number() as i64);
                    Value::Number(match operator {
                        "and" => a & b,
                        "or" => a | b,
                        _ => a ^ b,
                    } as f64)
                }
            };
            stack.push(value);
        }
        "not" => match stack.pop() {
            Some(Value::Bool(value)) => stack.push(Value::Bool(!value)),
            Some(Value::Number(value)) => stack.push(Value::Number(!(value as i64) as f64)),
            None => {}
        },
        "pop" => {
            stack.pop();
        }
        "exch" => {
            let length = stack.len();
            if length >= 2 {
                stack.swap(length - 1, length - 2);
            }
        }
        "dup" => {
            if let Some(&top) = stack.last() {
                stack.push(top);
            }
        }
        "copy" => {
            let count = pop() as usize;
            if count <= stack.len() {
                let start = stack.len() - count;
                stack.extend_from_within(start..);
            }
        }
        "index" => {
            let depth = pop() as usize;
            if depth < stack.len() {
                let value = stack[stack.len() - 1 - depth];
                stack.push(value);
            }
        }
        "roll" => {
            let shift = pop() as i64;
            let count = pop() as usize;
            if count > 0 && count <= stack.len() {
                let start = stack.len() - count;
                let amount = shift.rem_euclid(count as i64) as usize;
                stack[start..].rotate_right(amount);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn function(source: &str, outputs: usize) -> PostScriptFunction {
        let bounds = Bounds { domain: vec![(0.0, 1.0); 2], range: Some(vec![(-100.0, 100.0); outputs]) };
        PostScriptFunction::read(bounds, source.as_bytes()).unwrap()
    }

    #[test]
    fn computes_arithmetic_and_stack_operations() {
        assert_eq!(function("{ add 2 mul }", 1).evaluate(&[0.25, 0.5]), vec![1.5]);
        assert_eq!(function("{ exch pop dup }", 2).evaluate(&[0.25, 0.5]), vec![0.5, 0.5]);
        assert_eq!(function("{ 7 3 1 roll }", 3).evaluate(&[1.0, 2.0]), vec![7.0, 1.0, 2.0]);
    }

    #[test]
    fn branches_with_if_and_ifelse() {
        let threshold = function("{ pop 0.5 gt { 1 } { 0 } ifelse }", 1);
        assert_eq!(threshold.evaluate(&[0.7, 0.0]), vec![1.0]);
        assert_eq!(threshold.evaluate(&[0.2, 0.0]), vec![0.0]);
        assert_eq!(function("{ pop dup 0.5 lt { pop 9 } if }", 1).evaluate(&[0.1, 0.0]), vec![9.0]);
    }

    #[test]
    fn rejects_programs_without_braces() {
        let bounds = Bounds { domain: vec![(0.0, 1.0)], range: None };
        assert!(PostScriptFunction::read(bounds.clone(), b"add").is_err());
        assert!(PostScriptFunction::read(bounds, b"{ add").is_err());
    }
}
