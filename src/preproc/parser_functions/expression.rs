/*
 * preproc/parser_functions/expression.rs
 *
 * ftml - Library to parse Wikidot text
 * Copyright (C) 2019-2026 Wikijump Team
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

use super::{WikidotParserFunctionOptions, WikidotZeroOperatorPolicy};

const MAX_EXPRESSION_BYTES: usize = 256;
const MAX_OPERATIONS: usize = 512;
const MAX_PARENTHESES: usize = 32;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ExpressionError {
    Invalid,
    UndefinedFunction(String),
    DivisionByZero,
    RemainderByZero,
}

impl ExpressionError {
    pub(super) fn runtime_message(&self) -> Option<String> {
        match self {
            Self::Invalid => None,
            Self::UndefinedFunction(name) => {
                Some(format!(r#"run-time error: undefined function "{name}""#))
            }
            Self::DivisionByZero => Some("run-time error: division by zero".to_owned()),
            Self::RemainderByZero => {
                Some("run-time error: rest-division by zero".to_owned())
            }
        }
    }
}

pub(super) fn evaluate(
    expression: &str,
    options: WikidotParserFunctionOptions,
) -> Result<f64, ExpressionError> {
    if expression.len() > MAX_EXPRESSION_BYTES || !expression.is_ascii() {
        return Err(ExpressionError::Invalid);
    }

    let mut parser = ExpressionParser {
        input: expression.as_bytes(),
        offset: 0,
        operations: 0,
        parentheses: 0,
        options,
    };
    let result = parser.parse_or()?;
    parser.skip_space();
    if parser.offset != parser.input.len() || !result.is_finite() {
        return Err(ExpressionError::Invalid);
    }
    Ok(result)
}

#[derive(Debug)]
struct ExpressionParser<'a> {
    input: &'a [u8],
    offset: usize,
    operations: usize,
    parentheses: usize,
    options: WikidotParserFunctionOptions,
}

impl ExpressionParser<'_> {
    fn parse_or(&mut self) -> Result<f64, ExpressionError> {
        let mut value = self.parse_and()?;
        while self.consume("||") {
            self.operation()?;
            let right = self.parse_and()?;
            value = f64::from(truthy(value) || truthy(right));
        }
        Ok(value)
    }

    fn parse_and(&mut self) -> Result<f64, ExpressionError> {
        let mut value = self.parse_comparison()?;
        while self.consume("&&") {
            self.operation()?;
            let right = self.parse_comparison()?;
            value = f64::from(truthy(value) && truthy(right));
        }
        Ok(value)
    }

    fn parse_comparison(&mut self) -> Result<f64, ExpressionError> {
        let left = self.parse_additive()?;
        let operator = [">=", "<=", "==", "!=", "=", ">", "<"]
            .into_iter()
            .find(|operator| self.consume(operator));
        let Some(operator) = operator else {
            return Ok(left);
        };
        self.operation()?;
        let right = self.parse_additive()?;
        let result = match operator {
            ">=" => left >= right,
            "<=" => left <= right,
            "=" | "==" => nearly_equal(left, right),
            "!=" => !nearly_equal(left, right),
            ">" => left > right,
            "<" => left < right,
            _ => unreachable!("comparison operator comes from fixed list"),
        };
        Ok(f64::from(result))
    }

    fn parse_additive(&mut self) -> Result<f64, ExpressionError> {
        let mut value = self.parse_multiplicative()?;
        loop {
            if self.consume("+") {
                self.operation()?;
                value += self.parse_multiplicative()?;
            } else if self.consume("-") {
                self.operation()?;
                value -= self.parse_multiplicative()?;
            } else {
                return Ok(value);
            }
        }
    }

    fn parse_multiplicative(&mut self) -> Result<f64, ExpressionError> {
        let mut value = self.parse_unary()?;
        loop {
            if self.consume("*") {
                self.operation()?;
                value *= self.parse_unary()?;
            } else if self.consume("/") {
                self.operation()?;
                let divisor = self.parse_unary()?;
                if divisor == 0.0 {
                    match self.options.zero_operator_policy {
                        WikidotZeroOperatorPolicy::RuntimeError => {
                            return Err(ExpressionError::DivisionByZero);
                        }
                        WikidotZeroOperatorPolicy::ReplaceOperationWithZero => {
                            value = 0.0;
                        }
                    }
                } else {
                    value /= divisor;
                }
            } else if self.consume("%") {
                self.operation()?;
                let divisor = self.parse_unary()?;
                if divisor == 0.0 {
                    match self.options.zero_operator_policy {
                        WikidotZeroOperatorPolicy::RuntimeError => {
                            return Err(ExpressionError::RemainderByZero);
                        }
                        WikidotZeroOperatorPolicy::ReplaceOperationWithZero => {
                            value = 0.0;
                        }
                    }
                } else {
                    value %= divisor;
                }
            } else {
                return Ok(value);
            }
        }
    }

    fn parse_unary(&mut self) -> Result<f64, ExpressionError> {
        if self.consume("+") {
            self.operation()?;
            self.parse_unary()
        } else if self.consume("-") {
            self.operation()?;
            Ok(-self.parse_unary()?)
        } else if self.consume("!") {
            self.operation()?;
            Ok(f64::from(!truthy(self.parse_unary()?)))
        } else {
            self.parse_primary()
        }
    }

    fn parse_primary(&mut self) -> Result<f64, ExpressionError> {
        if self.consume("(") {
            self.parentheses += 1;
            if self.parentheses > MAX_PARENTHESES {
                return Err(ExpressionError::Invalid);
            }
            let value = self.parse_or()?;
            if !self.consume(")") {
                return Err(ExpressionError::Invalid);
            }
            self.parentheses -= 1;
            return Ok(value);
        }

        self.skip_space();
        if self
            .input
            .get(self.offset)
            .is_some_and(u8::is_ascii_alphabetic)
        {
            return self.parse_function();
        }
        self.parse_number()
    }

    fn parse_function(&mut self) -> Result<f64, ExpressionError> {
        self.skip_space();
        let start = self.offset;
        while self
            .input
            .get(self.offset)
            .is_some_and(u8::is_ascii_alphabetic)
        {
            self.offset += 1;
        }
        let name = &self.input[start..self.offset];
        if name.eq_ignore_ascii_case(b"true") {
            return Ok(1.0);
        }
        if name.eq_ignore_ascii_case(b"false") {
            return Ok(0.0);
        }
        if !self.consume("(") {
            return Err(ExpressionError::Invalid);
        }

        self.parentheses += 1;
        if self.parentheses > MAX_PARENTHESES {
            return Err(ExpressionError::Invalid);
        }
        let mut arguments = vec![self.parse_or()?];
        while self.consume(",") {
            arguments.push(self.parse_or()?);
        }
        if !self.consume(")") {
            return Err(ExpressionError::Invalid);
        }
        self.parentheses -= 1;
        self.operation()?;

        match name {
            name if name.eq_ignore_ascii_case(b"abs") && arguments.len() == 1 => {
                Ok(arguments[0].abs())
            }
            name if name.eq_ignore_ascii_case(b"min") => arguments
                .into_iter()
                .reduce(f64::min)
                .ok_or(ExpressionError::Invalid),
            name if name.eq_ignore_ascii_case(b"max") => arguments
                .into_iter()
                .reduce(f64::max)
                .ok_or(ExpressionError::Invalid),
            name if matches_ignore_ascii_case(name, &[b"abs", b"min", b"max"]) => {
                Err(ExpressionError::Invalid)
            }
            _ => Err(ExpressionError::UndefinedFunction(
                String::from_utf8_lossy(name).into_owned(),
            )),
        }
    }

    fn parse_number(&mut self) -> Result<f64, ExpressionError> {
        self.skip_space();
        let start = self.offset;
        let mut decimal = false;
        while let Some(byte) = self.input.get(self.offset) {
            if byte.is_ascii_digit() {
                self.offset += 1;
            } else if *byte == b'.' && !decimal {
                decimal = true;
                self.offset += 1;
            } else {
                break;
            }
        }
        if start == self.offset || self.input[start..self.offset] == *b"." {
            return Err(ExpressionError::Invalid);
        }
        std::str::from_utf8(&self.input[start..self.offset])
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| value.is_finite())
            .ok_or(ExpressionError::Invalid)
    }

    fn consume(&mut self, expected: &str) -> bool {
        self.skip_space();
        if self.input[self.offset..].starts_with(expected.as_bytes()) {
            self.offset += expected.len();
            true
        } else {
            false
        }
    }

    fn skip_space(&mut self) {
        while self
            .input
            .get(self.offset)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.offset += 1;
        }
    }

    fn operation(&mut self) -> Result<(), ExpressionError> {
        self.operations += 1;
        if self.operations > MAX_OPERATIONS {
            Err(ExpressionError::Invalid)
        } else {
            Ok(())
        }
    }
}

fn matches_ignore_ascii_case(value: &[u8], choices: &[&[u8]]) -> bool {
    choices
        .iter()
        .any(|choice| value.eq_ignore_ascii_case(choice))
}

pub(super) fn truthy(value: f64) -> bool {
    value != 0.0
}

fn nearly_equal(left: f64, right: f64) -> bool {
    (left - right).abs() <= f64::EPSILON
}

pub(super) fn format_value(value: f64) -> String {
    if value == 0.0 {
        return "0".to_owned();
    }
    let mut output = format!("{value:.11}");
    while output.ends_with('0') {
        output.pop();
    }
    if output.ends_with('.') {
        output.pop();
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_precedence_functions_and_boolean_operators() {
        let options = WikidotParserFunctionOptions::default();
        assert_eq!(evaluate("2*(2-1)", options), Ok(2.0));
        assert_eq!(evaluate("abs(-100)", options), Ok(100.0));
        assert_eq!(evaluate("min(4,1,-4,6,-10)", options), Ok(-10.0));
        assert_eq!(evaluate("max(4,1,-4,6,-10)", options), Ok(6.0));
        assert_eq!(evaluate("0 || (1 && !0)", options), Ok(1.0));
        assert_eq!(evaluate("1 || 0", options), Ok(1.0));
        assert_eq!(evaluate("0 && 1", options), Ok(0.0));
    }

    #[test]
    fn evaluates_ordinary_operator_and_literal_boundaries() {
        let options = WikidotParserFunctionOptions::default();

        for (expression, expected) in [
            ("1<2", 1.0),
            ("8/2", 4.0),
            ("5%2", 1.0),
            ("+2", 2.0),
            ("true + false", 1.0),
            ("1.5 + .5", 2.0),
        ] {
            assert_eq!(evaluate(expression, options), Ok(expected), "{expression}");
        }
        assert_eq!(format_value(0.0), "0");
    }

    #[test]
    fn rejects_malformed_depth_and_operation_boundaries() {
        let options = WikidotParserFunctionOptions::default();
        let deep_parentheses = format!(
            "{}1{}",
            "(".repeat(MAX_PARENTHESES + 1),
            ")".repeat(MAX_PARENTHESES + 1),
        );
        let deep_functions = format!(
            "{}1{}",
            "abs(".repeat(MAX_PARENTHESES + 1),
            ")".repeat(MAX_PARENTHESES + 1),
        );

        for expression in [
            "1 2",
            "(1",
            "abs(1",
            ".",
            deep_parentheses.as_str(),
            deep_functions.as_str(),
        ] {
            assert_eq!(
                evaluate(expression, options),
                Err(ExpressionError::Invalid),
                "{expression}",
            );
        }

        let mut parser = ExpressionParser {
            input: b"",
            offset: 0,
            operations: MAX_OPERATIONS,
            parentheses: 0,
            options,
        };
        assert_eq!(parser.operation(), Err(ExpressionError::Invalid));
    }

    #[test]
    fn distinguishes_zero_operator_errors() {
        let options = WikidotParserFunctionOptions::default();
        assert_eq!(
            evaluate("1/0+1", options),
            Err(ExpressionError::DivisionByZero),
        );
        assert_eq!(
            evaluate("1%0", options),
            Err(ExpressionError::RemainderByZero),
        );
    }

    #[test]
    fn optional_zero_policy_replaces_only_the_zero_operator_result() {
        let options = WikidotParserFunctionOptions {
            zero_operator_policy: WikidotZeroOperatorPolicy::ReplaceOperationWithZero,
        };
        assert_eq!(evaluate("1/0+1", options), Ok(1.0));
        assert_eq!(evaluate("5%0+2", options), Ok(2.0));
    }

    #[test]
    fn invalid_and_bounded_expressions_are_distinct_from_runtime_errors() {
        let options = WikidotParserFunctionOptions::default();
        assert_eq!(evaluate("abs(1,2)", options), Err(ExpressionError::Invalid));
        assert_eq!(
            evaluate("unknown(1)", options),
            Err(ExpressionError::UndefinedFunction("unknown".to_owned())),
        );
        assert_eq!(
            evaluate(&"1+".repeat(129), options),
            Err(ExpressionError::Invalid),
        );
    }
}
