/*
 * parsing/rule/impls/block/blocks/math.rs
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
use std::borrow::Cow;

pub const BLOCK_MATH: BlockRule = BlockRule {
    name: "block-math",
    accepts_names: &["math"],
    accepts_star: false,
    accepts_score: false,
    accepts_newlines: true,
    parse_fn,
};

fn parse_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Parsing math block (name '{name}', in-head {in_head})");
    assert!(!flag_star, "User doesn't allow star flag");
    assert!(!flag_score, "User doesn't allow score flag");
    assert_block_name(&BLOCK_MATH, name);

    if parser.settings().layout.legacy() && parser.native_blockquote_depth().is_some() {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    let name = get_math_head(parser, in_head)?;
    if parser.settings().layout.legacy()
        && name.as_deref().is_some_and(|name| !wikidot_math_name(name))
    {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    let latex_source = match parser.get_body_text(&BLOCK_MATH)? {
        Cow::Borrowed(source) => Cow::Borrowed(source.trim()),
        Cow::Owned(source) => Cow::Owned(source.trim().to_owned()),
    };
    if parser.settings().layout.legacy()
        && wikidot_math_crosses_bold_owner(parser, latex_source.as_ref())
    {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    if latex_source.is_empty() && !parser.settings().layout.legacy() {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    let element = Element::Math { name, latex_source };

    success_elements(element)
}

fn get_math_head<'r, 't>(
    parser: &mut Parser<'r, 't>,
    in_head: bool,
) -> Result<Option<Cow<'t, str>>, ParseError>
where
    'r: 't,
{
    if parser.settings().layout.legacy() && in_head {
        let mut attribute_candidate = parser.clone();
        if let Ok(arguments) = attribute_candidate.get_head_map(&BLOCK_MATH, in_head)
            && arguments.has_source()
            && !arguments.has_bare_source()
        {
            parser.update(&attribute_candidate);
            return Ok(None);
        }
    }

    parser.get_head_value(&BLOCK_MATH, in_head, |_, value| {
        Ok(value.map(|source| Cow::Borrowed(source.trim())))
    })
}

fn wikidot_math_crosses_bold_owner(parser: &Parser<'_, '_>, body: &str) -> bool {
    if wikidot_unclosed_block_depth(body, "bold") == 0 {
        return false;
    }

    let suffix = &parser.full_text().inner()[parser.current().span.start..];
    let suffix = suffix.trim_start_matches([' ', '\t', '\r', '\n', '\0']);
    let Some(close) = suffix.strip_prefix("[[/") else {
        return false;
    };
    let Some(end) = close.find("]]") else {
        return false;
    };
    close[..end].trim().eq_ignore_ascii_case("bold")
}

fn wikidot_unclosed_block_depth(source: &str, block_name: &str) -> usize {
    let mut depth = 0_usize;
    let mut remaining = source;

    while let Some(start) = remaining.find("[[") {
        remaining = &remaining[start + 2..];
        let Some(end) = remaining.find("]]") else {
            break;
        };
        let marker = remaining[..end].trim();
        let marker_name = marker.split_ascii_whitespace().next().unwrap_or_default();
        if marker_name.eq_ignore_ascii_case(block_name) {
            depth += 1;
        } else if marker_name
            .strip_prefix('/')
            .is_some_and(|name| name.eq_ignore_ascii_case(block_name))
        {
            depth = depth.saturating_sub(1);
        }
        remaining = &remaining[end + 2..];
    }

    depth
}

pub(crate) fn wikidot_math_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[cfg(test)]
mod tests {
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::{Render, html::HtmlRender};
    use crate::settings::{WikitextMode, WikitextSettings};

    fn render(layout: Layout, source: &str) -> (String, Vec<crate::parsing::ParseError>) {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, layout);
        let mut source = source.to_owned();
        crate::preprocess(&mut source);
        let tokenization = crate::tokenize(&source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        (HtmlRender.render(&tree, &page_info, &settings).body, errors)
    }

    #[test]
    fn quoted_math_block_trims_owned_source() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);
        let tokenization = crate::tokenize(concat!(
            "> [[collapsible]]\n",
            "> [[math]]\n",
            ">   x + y   \n",
            "> [[/math]]\n",
            "> [[/collapsible]]\n",
        ));
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");
        assert!(format!("{tree:?}").contains("x + y"));
    }

    #[test]
    fn wikidot_math_preserves_live_block_boundaries_and_dom() {
        let source = concat!(
            "[[math]] A [[/math]]\n\n",
            "[[math]]\nB\n[[/math]]\n\n",
            "[[math invalid-name]]\n\\frac{x}{y}\n[[/math]]\n\n",
            "[[math valid_name]]\nC\n[[/math]]\n",
            "X[[eref valid_name]]Y[[eref missing]]Z\n\n",
            "[[math]]\n[[/math]]",
        );
        let (html, errors) = render(Layout::Wikidot, source);

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(html.contains(
            r#"<span class="equation-number">(1)</span>
<div class="math-equation" id="equation-1">\begin{align} A [[/math]] [[math]] B [[/math]] [[math invalid-name]] \frac{x}{y} \end{align}</div>"#,
        ), "{html}");
        assert!(
            html.contains(
                r#"<span class="equation-number">(2)</span>
<div class="math-equation" id="equation-2">\begin{equation} C \end{equation}</div>"#,
            ),
            "{html}"
        );
        assert!(html.contains(
            r#"X<a class="eref" href="javascript:;" onclick="WIKIDOT.page.utils.scrollToReference(&#39;equation-2&#39;)">2</a>Y<a class="eref" href="javascript:;" onclick="WIKIDOT.page.utils.scrollToReference(&#39;equation-&#39;)"></a>Z"#,
        ), "{html}");
        assert!(
            html.contains(
                r#"<span class="equation-number">(3)</span>
<div class="math-equation" id="equation-3">\begin{equation} \end{equation}</div>"#,
            ),
            "{html}"
        );
    }

    #[test]
    fn wikijump_math_rendering_is_unchanged() {
        let (html, errors) = render(
            Layout::Wikijump,
            "[[math named-equation]]\nx + y\n[[/math]]",
        );

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(html.contains(r#"class="wj-math wj-math-block""#), "{html}");
        assert!(html.contains(r#"data-name="named-equation""#), "{html}");
        assert!(!html.contains(r#"class="math-equation""#), "{html}");
    }
}
