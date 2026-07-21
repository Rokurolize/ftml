/*
 * preproc/parser_functions/mod.rs
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

//! Context-free Wikidot parser-function orchestration.

mod expression;
mod literal;

use self::expression::{evaluate, format_value, truthy};
use self::literal::LiteralRegionIndex;
use regex::Regex;
use std::ops::Range;
use std::sync::LazyLock;

const MAX_RESOLUTION_PASSES: usize = 32;
const MAX_DOCUMENT_CANDIDATES: usize = 8_192;
const MAX_CONDITIONAL_SCAN_MULTIPLIER: usize = 32;

static CONDITIONAL_OPEN_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)\[\[#(?P<kind>ifexpr|if)\s+").unwrap());
static EXPR_OPEN_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)\[\[#expr\s+").unwrap());

/// Policy for arithmetic division or remainder operations with a zero divisor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WikidotZeroOperatorPolicy {
    /// Emit Wikidot's runtime error for a zero divisor.
    #[default]
    RuntimeError,

    /// Replace that operator's result with zero, then continue evaluation.
    ///
    /// This supports callers with an independently evidenced compatibility
    /// policy without maintaining a second expression evaluator.
    ReplaceOperationWithZero,
}

/// Context-free parser-function evaluation options.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WikidotParserFunctionOptions {
    /// Behavior of `/` and `%` when their right-hand operand is zero.
    pub zero_operator_policy: WikidotZeroOperatorPolicy,
}

/// Resolve parser functions using Wikidot's generic runtime-error behavior.
///
/// This targeted entry point can be called before include expansion. Functions
/// in code, HTML, raw, and escaped regions remain byte-preserving. Parser
/// functions inside initial comments are evaluated because Wikidot uses them
/// to generate comment delimiters before final comment parsing.
/// Invalid or resource-bounded expressions remain literal.
pub fn resolve_wikidot_parser_functions(source: &str) -> String {
    resolve_wikidot_parser_functions_with_options(
        source,
        WikidotParserFunctionOptions::default(),
    )
}

/// Resolve parser functions with an explicit context-free evaluation policy.
///
/// At most 8,192 parser-function candidates, 32 nested passes, and a
/// document-proportional amount of malformed-conditional scan work are
/// examined per document. Content beyond those bounds remains literal.
pub fn resolve_wikidot_parser_functions_with_options(
    source: &str,
    options: WikidotParserFunctionOptions,
) -> String {
    let mut resolved = source.to_owned();
    let mut budget = CandidateBudget::default();
    let mut scan_budget = ConditionalScanBudget::new(source.len());

    for _ in 0..MAX_RESOLUTION_PASSES {
        let conditional =
            resolve_conditional_pass(&resolved, options, &mut budget, &mut scan_budget);
        if conditional != resolved {
            resolved = conditional;
            if budget.exhausted() {
                break;
            }
            continue;
        }
        if budget.exhausted() {
            break;
        }

        let expression = resolve_expression_pass(&resolved, options, &mut budget);
        if expression == resolved {
            break;
        }
        resolved = expression;
        if budget.exhausted() {
            break;
        }
    }

    resolved
}

pub(super) fn substitute(text: &mut String) {
    if !text.contains("[[#") {
        return;
    }
    *text = resolve_wikidot_parser_functions(text);
}

#[derive(Debug)]
struct CandidateBudget {
    remaining: usize,
}

#[derive(Debug)]
struct ConditionalScanBudget {
    remaining: usize,
}

impl Default for CandidateBudget {
    fn default() -> Self {
        Self {
            remaining: MAX_DOCUMENT_CANDIDATES,
        }
    }
}

impl ConditionalScanBudget {
    fn new(source_len: usize) -> Self {
        Self {
            remaining: source_len.saturating_mul(MAX_CONDITIONAL_SCAN_MULTIPLIER),
        }
    }

    fn take(&mut self) -> bool {
        if self.remaining == 0 {
            return false;
        }
        self.remaining -= 1;
        true
    }
}

impl CandidateBudget {
    fn take(&mut self) -> bool {
        if self.remaining == 0 {
            return false;
        }
        self.remaining -= 1;
        true
    }

    fn exhausted(&self) -> bool {
        self.remaining == 0
    }
}

#[derive(Debug)]
enum ConditionalSearch {
    Found(ConditionalParts),
    NotFound,
    Exhausted,
}

#[derive(Debug)]
struct ConditionalParts {
    end: usize,
    condition: Range<usize>,
    when_true: Range<usize>,
    when_false: Option<Range<usize>>,
}

fn resolve_conditional_pass(
    source: &str,
    options: WikidotParserFunctionOptions,
    budget: &mut CandidateBudget,
    scan_budget: &mut ConditionalScanBudget,
) -> String {
    let literal_regions = LiteralRegionIndex::new(source);
    let mut replacements = Vec::new();
    let mut search_start = 0usize;

    while let Some(captures) = CONDITIONAL_OPEN_REGEX.captures(&source[search_start..]) {
        if !budget.take() {
            break;
        }

        let full_open = captures.get(0).expect("conditional opening capture exists");
        let function_start = search_start + full_open.start();
        let condition_start = search_start + full_open.end();
        let kind = captures
            .name("kind")
            .expect("conditional kind capture exists")
            .as_str();
        let parts = match find_conditional_parts(source, condition_start, scan_budget) {
            ConditionalSearch::Found(parts) => parts,
            ConditionalSearch::NotFound => {
                search_start = condition_start;
                continue;
            }
            ConditionalSearch::Exhausted => break,
        };

        if literal_regions.contains(function_start) {
            search_start = parts.end;
            continue;
        }

        let condition = source[parts.condition.clone()].trim();
        let truth = if kind.eq_ignore_ascii_case("ifexpr") {
            match evaluate(condition, options) {
                Ok(value) => Some(Ok(truthy(value))),
                Err(error) => error.runtime_message().map(Err),
            }
        } else {
            Some(Ok(simple_condition(condition)))
        };

        let Some(truth) = truth else {
            // Continue inside an invalid outer function so a valid nested
            // function can still be resolved in this bounded pass.
            search_start = condition_start;
            continue;
        };
        let replacement = match truth {
            Ok(true) => source[parts.when_true].trim().to_owned(),
            Ok(false) => parts
                .when_false
                .map_or("", |range| &source[range])
                .trim()
                .to_owned(),
            Err(message) => message,
        };
        replacements.push((function_start..parts.end, replacement));
        search_start = parts.end;
    }

    apply_replacements(source, replacements)
}

fn simple_condition(condition: &str) -> bool {
    !condition.is_empty() && condition != "0" && !condition.eq_ignore_ascii_case("false")
}

fn find_conditional_parts(
    source: &str,
    condition_start: usize,
    scan_budget: &mut ConditionalScanBudget,
) -> ConditionalSearch {
    let bytes = source.as_bytes();
    let mut cursor = condition_start;
    let mut depth = 1usize;
    let mut separators = [None, None];

    while cursor + 1 < bytes.len() {
        if !scan_budget.take() {
            return ConditionalSearch::Exhausted;
        }
        if bytes[cursor..].starts_with(b"[[") {
            depth += 1;
            cursor += 2;
            continue;
        }
        if bytes[cursor..].starts_with(b"]]") {
            if depth == 1 {
                let Some(first) = separators[0] else {
                    return ConditionalSearch::NotFound;
                };
                let true_end = separators[1].unwrap_or(cursor);
                return ConditionalSearch::Found(ConditionalParts {
                    end: cursor + 2,
                    condition: condition_start..first,
                    when_true: first + 1..true_end,
                    when_false: separators[1].map(|second| second + 1..cursor),
                });
            }
            depth -= 1;
            cursor += 2;
            continue;
        }
        if depth == 1 && bytes[cursor] == b'|' {
            if bytes.get(cursor + 1) == Some(&b'|') {
                cursor += 2;
                continue;
            }
            if separators[0].is_none() {
                separators[0] = Some(cursor);
            } else if separators[1].is_none() {
                separators[1] = Some(cursor);
            }
        }
        cursor += 1;
    }
    ConditionalSearch::NotFound
}

fn resolve_expression_pass(
    source: &str,
    options: WikidotParserFunctionOptions,
    budget: &mut CandidateBudget,
) -> String {
    let literal_regions = LiteralRegionIndex::new(source);
    let mut replacements = Vec::new();
    let mut search_start = 0usize;

    while let Some(open) = EXPR_OPEN_REGEX.find(&source[search_start..]) {
        if !budget.take() {
            break;
        }

        let function_start = search_start + open.start();
        let expression_start = search_start + open.end();
        let Some(relative_end) = source[expression_start..].find("]]") else {
            break;
        };
        let close_start = expression_start + relative_end;
        let function_end = close_start + 2;
        search_start = function_end;

        if literal_regions.contains(function_start) {
            continue;
        }

        let original = &source[function_start..function_end];
        let replacement =
            match evaluate(source[expression_start..close_start].trim(), options) {
                Ok(value) => format_value(value),
                Err(error) => error
                    .runtime_message()
                    .unwrap_or_else(|| original.to_owned()),
            };
        if replacement != original {
            replacements.push((function_start..function_end, replacement));
        }
    }

    apply_replacements(source, replacements)
}

fn apply_replacements(source: &str, replacements: Vec<(Range<usize>, String)>) -> String {
    if replacements.is_empty() {
        return source.to_owned();
    }

    let mut resolved = source.to_owned();
    for (range, replacement) in replacements.into_iter().rev() {
        resolved.replace_range(range, &replacement);
    }
    resolved
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_expr_ifexpr_and_simple_if() {
        let source = concat!(
            "[[#expr 7*6]] ",
            "[[#ifexpr 2*(2-1) == 2 | true branch | false branch]] ",
            "[[#if 0 | hidden | shown]]",
        );

        assert_eq!(
            resolve_wikidot_parser_functions(source),
            "42 true branch shown",
        );
    }

    #[test]
    fn simple_if_uses_wikidot_nonempty_string_truthiness() {
        // Live provenance:
        // ftml-oracle-20260712T225511Z/run-parser-if-string and
        // ftml-oracle-20260712T225812Z/run-parser-if-include.
        for (condition, expected) in [
            ("foo", "true branch"),
            ("no", "true branch"),
            ("aroace", "true branch"),
            ("{$code}", "true branch"),
            ("", "false branch"),
            ("0", "false branch"),
            ("false", "false branch"),
            ("FALSE", "false branch"),
        ] {
            let source = format!("[[#if {condition} | true branch | false branch]]",);
            assert_eq!(
                resolve_wikidot_parser_functions(&source),
                expected,
                "{condition:?}",
            );
        }
    }

    #[test]
    fn balances_wikidot_markup_and_nested_parser_functions_in_branches() {
        let source = concat!(
            "[[#ifexpr 1 | [[span data-value=\"a|b\"]]shown[[/span]] | hidden]]",
            "[[#if 1 | [[#ifexpr 0 | no | nested]] | outer-hidden]]",
        );

        assert_eq!(
            resolve_wikidot_parser_functions(source),
            "[[span data-value=\"a|b\"]]shown[[/span]]nested",
        );
    }

    #[test]
    fn preserves_unclosed_outer_conditionals_but_resolves_nested_functions() {
        let source = concat!(
            "[[#ifexpr 0 || 1 | chosen | hidden]] ",
            "[[#if 1 | open [[#if 1 | nested | no]]",
        );

        assert_eq!(
            resolve_wikidot_parser_functions(source),
            "chosen [[#if 1 | open nested",
        );
    }

    #[test]
    fn preserves_parser_functions_in_literal_regions() {
        let source = concat!(
            "[[code]]\n[[#expr 1+1]]\n[[/code]]\n",
            "> [[html]]\n> [[#ifexpr 1 | html | hidden]]\n> [[/html]]\n",
            "[[raw]]\n[[#if 1 | raw | hidden]]\n[[/raw]]\n",
            "@@[[#expr 2+2]]@@\n",
            "[!-- [[#if 1 | comment | hidden]] --]\n",
            "[[code]]same-line[[/code]][[#expr 4+4]]\n",
            "[[#expr 3+3]]",
        );

        assert_eq!(
            resolve_wikidot_parser_functions(source),
            concat!(
                "[[code]]\n[[#expr 1+1]]\n[[/code]]\n",
                "> [[html]]\n> [[#ifexpr 1 | html | hidden]]\n> [[/html]]\n",
                "[[raw]]\n[[#if 1 | raw | hidden]]\n[[/raw]]\n",
                "@@[[#expr 2+2]]@@\n",
                "[!-- comment --]\n",
                "[[code]]same-line[[/code]]8\n",
                "6",
            ),
        );
    }

    #[test]
    fn emits_exact_generic_runtime_errors() {
        for (source, expected) in [
            (
                "[[#expr unknown(1)]]",
                r#"run-time error: undefined function "unknown""#,
            ),
            ("[[#expr 1/0]]", "run-time error: division by zero"),
            ("[[#expr 1/0+1]]", "run-time error: division by zero"),
            ("[[#expr 1%0]]", "run-time error: rest-division by zero"),
            (
                "[[#ifexpr 1/0 | leaked | hidden]]",
                "run-time error: division by zero",
            ),
        ] {
            assert_eq!(resolve_wikidot_parser_functions(source), expected);
        }
    }

    #[test]
    fn options_api_supports_caller_evidenced_zero_operator_policy() {
        let options = WikidotParserFunctionOptions {
            zero_operator_policy: WikidotZeroOperatorPolicy::ReplaceOperationWithZero,
        };
        assert_eq!(
            resolve_wikidot_parser_functions_with_options(
                "[[#expr 1/0+1]] [[#expr 5%0+2]]",
                options,
            ),
            "1 2",
        );
    }

    #[test]
    fn unverified_invalid_inputs_fail_closed() {
        for source in [
            "[[#expr abs(1,2)]]",
            "[[#ifexpr missing | leaked | hidden]]",
        ] {
            assert_eq!(resolve_wikidot_parser_functions(source), source);
        }
    }

    #[test]
    fn expression_and_nested_resolution_limits_fail_closed() {
        let overlong = format!("[[#expr {}]]", "1+".repeat(129));
        assert_eq!(resolve_wikidot_parser_functions(&overlong), overlong);

        let mut nested = "leaf".to_owned();
        for _ in 0..(MAX_RESOLUTION_PASSES + 8) {
            nested = format!("[[#if 1 | {nested} | hidden]]");
        }
        let resolved = resolve_wikidot_parser_functions(&nested);
        assert_eq!(resolved.matches("[[#if").count(), 8);
        assert!(resolved.contains("leaf"));
    }

    #[test]
    fn document_candidate_limit_is_deterministic() {
        let source = "[[#expr 1]]".repeat(MAX_DOCUMENT_CANDIDATES + 1);
        let resolved = resolve_wikidot_parser_functions(&source);

        assert_eq!(resolved.matches("[[#expr 1]]").count(), 1);
        assert_eq!(resolved.matches('1').count(), MAX_DOCUMENT_CANDIDATES + 1);
        assert!(resolved.ends_with("[[#expr 1]]"));
    }

    #[test]
    fn malformed_conditional_scan_work_is_document_bounded() {
        let malformed = "[[#if ".repeat(MAX_DOCUMENT_CANDIDATES);
        let valid = "[[#if 1 | selected | hidden]]";
        let source = format!("{malformed}{valid}");

        let resolved = resolve_wikidot_parser_functions(&source);

        assert_eq!(resolved, source);
    }

    #[test]
    fn standard_preprocess_resolves_before_quote_compatibility_and_typography() {
        let mut source =
            concat!(">[[#expr 7*6]]\n", "> [[#if 1 | ``selected'' | hidden]]\n",)
                .to_owned();

        crate::preprocess(&mut source);

        assert_eq!(source, "\n> “selected”");
    }
}
