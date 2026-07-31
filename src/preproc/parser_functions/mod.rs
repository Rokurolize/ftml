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
pub(in crate::preproc) use self::literal::LiteralRegionIndex;
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
    CrossingLink(ConditionalParts),
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
        let (parts, crossing_link) =
            match find_conditional_parts(source, condition_start, scan_budget) {
                ConditionalSearch::Found(parts) => (parts, false),
                ConditionalSearch::CrossingLink(parts) => (parts, true),
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
        if crossing_link
            && (!kind.eq_ignore_ascii_case("ifexpr") || !matches!(&truth, Ok(false)))
        {
            // Only the provenance-backed false ifexpr shape may share the
            // inline link's closing delimiter. Unsupported outcomes remain
            // literal while independently valid nested functions may resolve.
            search_start = condition_start;
            continue;
        }
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
    let mut true_branch_whitespace_only = true;
    let mut crossing_link_depth = None;
    let mut crossing_link_fallback = None;
    let mut crossing_link_confirmed = false;
    let mut expected_adjacent_conditional = None;
    let mut adjacent_conditional_depth = None;
    let mut adjacent_conditional_has_separator = false;

    while cursor < bytes.len() {
        if !scan_budget.take() {
            return ConditionalSearch::Exhausted;
        }
        if bytes[cursor..].starts_with(b"[[") {
            let next_depth = depth + 1;
            if expected_adjacent_conditional == Some(cursor) {
                adjacent_conditional_depth = Some(next_depth);
                expected_adjacent_conditional = None;
            }
            let in_true_branch =
                depth == 1 && separators[0].is_some() && separators[1].is_none();
            let crossing_link = in_true_branch
                && true_branch_whitespace_only
                && crossing_link_fallback.is_none()
                && is_wikidot_crossing_link_opener(&source[cursor..]);
            if in_true_branch {
                true_branch_whitespace_only = false;
            }
            depth = next_depth;
            if crossing_link {
                crossing_link_depth = Some(depth);
            }
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
            if crossing_link_depth == Some(depth)
                && let Some(first) = separators[0]
            {
                // Wikidot permits the final `]]` of an inline `[[a ...]]` or
                // `[[/a ...]]` branch to close the surrounding conditional as
                // well. Prefer an ordinary balanced outer close whenever one
                // exists; this fallback is used only if the remaining source
                // never closes the conditional.
                let close_end = cursor + 2;
                if close_end == source.len()
                    || is_wikidot_conditional_opener(&source[close_end..])
                {
                    crossing_link_fallback = Some(ConditionalParts {
                        end: close_end,
                        condition: condition_start..first,
                        when_true: first + 1..close_end,
                        when_false: None,
                    });
                    if close_end == source.len() {
                        crossing_link_confirmed = true;
                    } else {
                        expected_adjacent_conditional = Some(close_end);
                    }
                }
            }
            if crossing_link_depth == Some(depth) {
                crossing_link_depth = None;
            }
            if adjacent_conditional_depth == Some(depth) {
                adjacent_conditional_depth = None;
                if adjacent_conditional_has_separator {
                    crossing_link_confirmed = true;
                }
                adjacent_conditional_has_separator = false;
            }
            depth -= 1;
            cursor += 2;
            continue;
        }
        if bytes[cursor] == b'|' {
            if bytes.get(cursor + 1) == Some(&b'|') {
                if depth == 1 && separators[0].is_some() && separators[1].is_none() {
                    true_branch_whitespace_only = false;
                }
                cursor += 2;
                continue;
            }
            if depth == 1 {
                if separators[0].is_none() {
                    separators[0] = Some(cursor);
                } else if separators[1].is_none() {
                    separators[1] = Some(cursor);
                }
            } else if adjacent_conditional_depth == Some(depth) {
                adjacent_conditional_has_separator = true;
            }
            cursor += 1;
            continue;
        }
        if depth == 1
            && separators[0].is_some()
            && separators[1].is_none()
            && !bytes[cursor].is_ascii_whitespace()
        {
            true_branch_whitespace_only = false;
        }
        cursor += 1;
    }
    if depth == 1
        && separators[1].is_none()
        && crossing_link_confirmed
        && expected_adjacent_conditional.is_none()
        && adjacent_conditional_depth.is_none()
    {
        crossing_link_fallback
            .map_or(ConditionalSearch::NotFound, ConditionalSearch::CrossingLink)
    } else {
        ConditionalSearch::NotFound
    }
}

fn is_wikidot_crossing_link_opener(source: &str) -> bool {
    ["[[a", "[[/a"].iter().any(|prefix| {
        source
            .get(..prefix.len())
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(prefix))
            && source
                .as_bytes()
                .get(prefix.len())
                .is_some_and(u8::is_ascii_whitespace)
    })
}

fn is_wikidot_conditional_opener(source: &str) -> bool {
    ["[[#ifexpr", "[[#if"].iter().any(|prefix| {
        source
            .get(..prefix.len())
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(prefix))
            && source
                .as_bytes()
                .get(prefix.len())
                .is_some_and(u8::is_ascii_whitespace)
    })
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
            "[[#if 1 | [[a href=\"/target\" | linked ]][[#if 1 | adjacent | no]] | hidden]]",
        );

        assert_eq!(
            resolve_wikidot_parser_functions(source),
            "[[span data-value=\"a|b\"]]shown[[/span]]nested[[a href=\"/target\" | linked ]]adjacent",
        );
    }

    #[test]
    fn resolves_crossing_listpages_link_conditionals() {
        // Live provenance: the exact ListPages body from
        // jp:old:scp-7884:L87:B2643, observed anonymously through
        // edit/PagePreviewModule on 2026-07-30. Source SHA-256:
        // 42e104fbe6b89be364fc99a331c59ddf8c368996650ff696c687fb54600f1916.
        // Runtime substitution makes each condition false and each
        // %%content{0}%% branch empty.
        let source = concat!(
            "[[#ifexpr 0-0>0 | [[a class=\"point\" href=\"https://scp-jp.wikidot.com/example\" ",
            "style=\"bottom: calc(172px + 0px * sin(56)); left: calc(172px + 0px * cos(56));\" | ",
            "]][[#ifexpr 0-0>0 | ] | ]][[#ifexpr 0-0>0 | ] | ]]",
            "[[#ifexpr 0-0>0 | [[/a | ]][[#ifexpr 0-0>0 | ] | ]][[#ifexpr 0-0>0 | ] | ]]",
        );

        assert_eq!(resolve_wikidot_parser_functions(source), "");
    }

    #[test]
    fn crossing_link_fallback_rejects_an_unclosed_explicit_false_branch() {
        for (source, expected) in [
            (
                concat!(
                    "[[#if 1 | selected | [[a href=\"/x\" | hidden ]]",
                    "[[#if 1 | adjacent | no]]",
                ),
                "[[#if 1 | selected | [[a href=\"/x\" | hidden ]]adjacent",
            ),
            (
                concat!(
                    "[[#if 0 | prefix [[a href=\"/x\" | hidden ]]",
                    "[[#if 1 | adjacent | no]]",
                ),
                "[[#if 0 | prefix [[a href=\"/x\" | hidden ]]adjacent",
            ),
            (
                concat!(
                    "[[#if 0 | [[a href=\"/target\" | linked ]]",
                    "[[#if 1 | nested | no]] | explicit false",
                ),
                "[[#if 0 | [[a href=\"/target\" | linked ]]nested | explicit false",
            ),
        ] {
            assert_eq!(resolve_wikidot_parser_functions(source), expected);
        }
    }

    #[test]
    fn crossing_link_fallback_requires_a_complete_adjacent_conditional() {
        let source = concat!(
            "[[#ifexpr 0 | [[a href=\"/x\" | hidden ]]",
            "[[#if 1 | open",
        );

        assert_eq!(resolve_wikidot_parser_functions(source), source);
    }

    #[test]
    fn crossing_link_must_be_the_first_structural_true_branch_token() {
        for (source, expected) in [
            (
                concat!(
                    "[[#ifexpr 0 | [[#if 1 | prefix | no]] ",
                    "[[a href=\"/x\" | hidden ]]",
                    "[[#if 1 | adjacent | no]]",
                ),
                "[[#ifexpr 0 | prefix [[a href=\"/x\" | hidden ]]adjacent",
            ),
            (
                concat!(
                    "[[#ifexpr 0 | [[a href=\"/one\" | first ]] ",
                    "[[a href=\"/two\" | second ]]",
                    "[[#if 1 | adjacent | no]]",
                ),
                concat!(
                    "[[#ifexpr 0 | [[a href=\"/one\" | first ]] ",
                    "[[a href=\"/two\" | second ]]adjacent",
                ),
            ),
        ] {
            assert_eq!(resolve_wikidot_parser_functions(source), expected);
        }
    }

    #[test]
    fn crossing_link_rejects_a_double_pipe_true_branch_prefix() {
        let source = concat!(
            "[[#ifexpr 0 | || [[a href=\"/x\" | hidden ]]",
            "[[#if 1 | adjacent | no]]",
        );

        assert_eq!(
            resolve_wikidot_parser_functions(source),
            "[[#ifexpr 0 | || [[a href=\"/x\" | hidden ]]adjacent",
        );
    }

    #[test]
    fn crossing_link_rejects_a_terminal_false_separator() {
        let source = concat!(
            "[[#ifexpr 0 | [[a href=\"/x\" | hidden ]]",
            "[[#if 1 | adjacent | no]]|",
        );

        assert_eq!(
            resolve_wikidot_parser_functions(source),
            "[[#ifexpr 0 | [[a href=\"/x\" | hidden ]]adjacent|",
        );
    }

    #[test]
    fn crossing_link_requires_a_structurally_valid_adjacent_conditional() {
        let source = concat!(
            "[[#ifexpr 0 | [[a href=\"/x\" | hidden ]]",
            "[[#if malformed]]",
        );

        assert_eq!(resolve_wikidot_parser_functions(source), source);
    }

    #[test]
    fn crossing_link_fallback_keeps_the_first_candidate() {
        let source = concat!(
            "[[#ifexpr 0 | [[a href=\"/one\" | first ]]",
            "[[#if 1 | adjacent | no]]",
            "[[a href=\"/two\" | outside ]]",
        );

        assert_eq!(
            resolve_wikidot_parser_functions(source),
            "adjacent[[a href=\"/two\" | outside ]]",
        );
    }

    #[test]
    fn crossing_link_lookahead_is_strictly_anchored() {
        assert!(is_wikidot_conditional_opener("[[#ifexpr 1 | yes | no]]"));
        assert!(is_wikidot_conditional_opener("[[#IF 1 | yes | no]]"));
        assert!(!is_wikidot_conditional_opener(
            "prefix [[#ifexpr 1 | yes | no]]",
        ));
        assert!(!is_wikidot_conditional_opener("[[#ifexpr]"));

        let links = "[[a href=\"/x\" | x ]]x".repeat(2_000);
        let source = format!("[[#ifexpr 0 | {links}");
        assert_eq!(resolve_wikidot_parser_functions(&source), source);
    }

    #[test]
    fn crossing_link_fallback_rejects_unevidenced_conditions() {
        for (source, expected) in [
            (
                concat!(
                    "[[#if 0 | [[a href=\"/x\" | hidden ]]",
                    "[[#if 1 | adjacent | no]]",
                ),
                "[[#if 0 | [[a href=\"/x\" | hidden ]]adjacent",
            ),
            (
                concat!(
                    "[[#ifexpr 1 | [[a href=\"/x\" | selected ]]",
                    "[[#if 1 | adjacent | no]]",
                ),
                "[[#ifexpr 1 | [[a href=\"/x\" | selected ]]adjacent",
            ),
            (
                concat!(
                    "[[#ifexpr 1/0 | [[a href=\"/x\" | error ]]",
                    "[[#if 1 | adjacent | no]]",
                ),
                "[[#ifexpr 1/0 | [[a href=\"/x\" | error ]]adjacent",
            ),
        ] {
            assert_eq!(resolve_wikidot_parser_functions(source), expected);
        }
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

        crate::preprocess_for_layout(&mut source, crate::layout::Layout::Wikidot);

        assert_eq!(source, "> “selected”");
    }
}
