/*
 * parsing/rule/impls/block/blocks/span.rs
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

use super::prelude::*;
use crate::delayed::DelayedElement;
use crate::parsing::strip_newlines;
use crate::tree::{PartialElement, WIKIDOT_GENERATED_EMPTY_CLASS_MARKER};

pub const BLOCK_SPAN: BlockRule = BlockRule {
    name: "block-span",
    accepts_names: &["span"],
    accepts_star: false,
    accepts_score: true,
    accepts_newlines: false,
    parse_fn,
};

fn parse_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Parsing span block (name '{name}', in-head {in_head})");
    assert!(!flag_star, "Span doesn't allow star flag");
    assert_block_name(&BLOCK_SPAN, name);

    let generated = parser.generated_until_right_block();
    if generated
        .iter()
        .any(|slot| slot.kind == crate::delayed::GeneratedKind::TagLinks)
    {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    let generated_attributes = generated
        .iter()
        .map(|slot| delayed_attribute_key(parser.full_text().inner(), slot))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| parser.make_err(ParseErrorKind::RuleFailed))?;
    if generated_attributes
        .iter()
        .any(|key| !matches!(*key, "class" | "title"))
    {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    let arguments = parser.get_head_map_wikidot(&BLOCK_SPAN, in_head)?;
    let has_close = parser.has_body_end_block(&BLOCK_SPAN);

    if parser.settings().layout.legacy() {
        if !has_close {
            return Err(parser.make_err(ParseErrorKind::RuleFailed));
        }
        let mut attributes = arguments.to_attribute_map(parser.settings());
        retain_wikidot_span_attributes(&mut attributes);
        for key in generated_attributes {
            match key {
                "title" => {
                    attributes.remove("title");
                }
                "class" => {
                    attributes.insert("class", cow!(""));
                    attributes.insert(WIKIDOT_GENERATED_EMPTY_CLASS_MARKER, cow!(""));
                }
                _ => unreachable!(),
            }
        }
        if flag_score {
            attributes.insert("data-ftml-score-span", cow!(""));
        }
        let span = Element::Partial(PartialElement::InlineSpanOpen(attributes));
        return if generated.is_empty() {
            ok!(span)
        } else {
            ok!(Elements::Multiple(vec![
                span,
                Element::Delayed(DelayedElement::omitted(&generated)),
            ]))
        };
    }

    // Get body content, without paragraphs
    let body = parser.get_body_elements(&BLOCK_SPAN, false)?;
    let (mut elements, errors, paragraph_safe) = body.into();

    if flag_score {
        strip_newlines(&mut elements);
    }

    let element = Element::Container(Container::new(
        ContainerType::Span,
        elements,
        arguments.to_attribute_map(parser.settings()),
    ));

    success_elements_with_paragraph_safety(paragraph_safe, element, errors)
}

fn retain_wikidot_span_attributes(attributes: &mut crate::tree::AttributeMap<'_>) {
    let rejected = attributes
        .get()
        .keys()
        .filter(|key| {
            !matches!(key.as_ref(), "class" | "id" | "style" | "role")
                && !key.starts_with("data-")
        })
        .cloned()
        .collect::<Vec<_>>();

    for key in rejected {
        attributes.remove(&key);
    }
}

fn delayed_attribute_key<'a>(
    source: &'a str,
    slot: &crate::delayed::GeneratedInput,
) -> Option<&'a str> {
    let prefix = &source[..slot.source_range.start];
    let equals = prefix.rfind('=')?;
    if !prefix[equals + 1..]
        .chars()
        .all(|character| matches!(character, '"' | '\''))
    {
        return None;
    }
    let key_start = prefix[..equals]
        .rfind(|character: char| {
            !(character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
        })
        .map_or(0, |position| position + 1);
    let key = &prefix[key_start..equals];
    (!key.is_empty()).then_some(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::Render;
    use crate::render::html::HtmlRender;
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn score_span_strips_line_breaks_from_body() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization =
            crate::tokenize("[[span_ class=\"compact\"]]\nalpha\n[[/span]]");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");
        let span = match tree.elements.as_slice() {
            [Element::Container(paragraph)] => paragraph
                .elements()
                .iter()
                .find_map(|element| match element {
                    Element::Container(container)
                        if container.ctype() == ContainerType::Span =>
                    {
                        Some(container)
                    }
                    _ => None,
                })
                .expect("paragraph should contain span container"),
            other => panic!("expected paragraph containing span, got {other:?}"),
        };

        assert_eq!(span.elements(), &[text!("alpha")]);
        assert_eq!(
            span.attributes()
                .get()
                .get("class")
                .map(|value| value.as_ref()),
            Some("compact"),
        );
    }

    #[test]
    fn wikidot_score_span_joins_only_its_adjacent_paragraph_boundaries() {
        let source = "PREVIOUS\n\nEMPTY [[span_]][[/span]]\nBASIC [[span_]]durian[[/span]]\n\nNEWLINES\n\n[[span_]]eggplant\nrafflesia[[/span]]\n\n[[span_]]\nparagraph\n\nin span\n[[/span]]\n\n[[span_ id=\"this-thing\"]]span2[[/span]]";
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");
        let html = HtmlRender.render(&tree, &page_info, &settings).body;
        assert_eq!(
            html,
            "<p>PREVIOUS</p><p>EMPTYBASIC <span>durian</span>NEWLINES<span>eggplant<br>\nrafflesia</span><span>paragraph</span></p><span>in span</span><span id=\"u-this-thing\">span2</span>",
        );
    }

    #[test]
    fn wikidot_span_does_not_duplicate_space_across_its_close_boundary() {
        let source = "[[span]]literal [[/span]] tail";
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");
        let html = HtmlRender.render(&tree, &page_info, &settings).body;
        assert_eq!(html, "<p><span>literal</span> tail</p>");
    }

    #[test]
    fn wikidot_empty_span_removes_its_preceding_space() {
        let source = "EMPTY [[span]][[/span]]\nBASIC [[span]]apple[[/span]]";
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");
        let html = HtmlRender.render(&tree, &page_info, &settings).body;
        assert_eq!(html, "<p>EMPTY<br>\nBASIC <span>apple</span></p>");
    }

    #[test]
    fn wikidot_span_scope_resumes_without_a_paragraph_after_div() {
        let source = "[[span style=\"color: rgb(1, 2, 3);\"]]SPAN_BEFORE\n[[div class=\"span-scope-block\"]]\nSPAN_INSIDE\n[[/div]]\nSPAN_AFTER[[/span]]";
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");
        let html = HtmlRender.render(&tree, &page_info, &settings).body;
        assert_eq!(
            html,
            "<p><span style=\"color: rgb(1, 2, 3);\">SPAN_BEFORE</span></p><div class=\"span-scope-block\"><p><span style=\"color: rgb(1, 2, 3);\">SPAN_INSIDE</span></p></div><span style=\"color: rgb(1, 2, 3);\"><br>\nSPAN_AFTER</span>",
        );
    }

    #[test]
    fn wikidot_span_scope_stays_open_across_unsupported_inline_aliases() {
        let source = "[[b]]Nested [[span]]blocks [[bold]][[span]]even[[strong]]more[[/strong]][[/span]][[/bold]][[/span]][[/b]]";
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(source);
        let (tree, _errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;
        assert_eq!(
            html,
            "<p>[[b]]Nested <span>blocks [[bold]]<span>even[[strong]]more[[/strong]]</span>[[/bold]]</span>[[/b]]</p>",
        );
    }

    #[test]
    fn wikidot_adjacent_span_scopes_remain_distinct_elements() {
        let source = "[[span]]one[[/span]][[span]]two[[/span]]";
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");
        let html = HtmlRender.render(&tree, &page_info, &settings).body;
        assert_eq!(html, "<p><span>one</span><span>two</span></p>");
    }
}
