use super::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum WikidotContent {
    Empty,
    FootnoteFirst,
    Other,
}

pub(super) fn starts_footnote(parser: &Parser<'_, '_>) -> bool {
    parser.current().token == Token::LeftBlock
        && parser
            .look_ahead(0)
            .is_some_and(|token| token.slice.eq_ignore_ascii_case("footnote"))
}

pub(super) fn unowned_footnote_first(
    parser: &Parser<'_, '_>,
    content_start: usize,
    content_end: usize,
    elements: &[Element<'_>],
) -> bool {
    if wikidot_content(elements) != WikidotContent::FootnoteFirst {
        return false;
    }

    let source = parser.full_text().inner();
    let Some(line) = source.get(content_start..content_end) else {
        return false;
    };
    let Some(footnote_start) = find_uncommented_footnote(line) else {
        return false;
    };

    !prefix_has_authored_syntax(&line[..footnote_start])
}

pub(super) fn remove_first_footnote(elements: &mut Vec<Element<'_>>) -> bool {
    let mut index = 0;
    while index < elements.len() {
        let content = match &elements[index] {
            Element::Text(text) | Element::Raw(text)
                if text.chars().all(char::is_whitespace) =>
            {
                WikidotContent::Empty
            }
            Element::Container(container) => wikidot_content(container.elements()),
            Element::Anchor { elements, .. } | Element::Color { elements, .. } => {
                wikidot_content(elements)
            }
            Element::Footnote(_) => WikidotContent::FootnoteFirst,
            Element::LineBreak => WikidotContent::Empty,
            Element::Partial(partial) if partial.is_inline_format_control() => {
                WikidotContent::Empty
            }
            _ => WikidotContent::Other,
        };

        match content {
            WikidotContent::Empty => index += 1,
            WikidotContent::FootnoteFirst => {
                if matches!(elements[index], Element::Footnote(_)) {
                    elements.remove(index);
                    return true;
                }
                return remove_first_footnote_from_element(&mut elements[index]);
            }
            WikidotContent::Other => return false,
        }
    }
    false
}

fn remove_first_footnote_from_element(element: &mut Element<'_>) -> bool {
    match element {
        Element::Container(container) => remove_first_footnote(container.elements_mut()),
        Element::Anchor { elements, .. } | Element::Color { elements, .. } => {
            remove_first_footnote(elements)
        }
        _ => false,
    }
}

fn wikidot_content(elements: &[Element<'_>]) -> WikidotContent {
    for element in elements {
        let content = match element {
            Element::Text(text) | Element::Raw(text)
                if text.chars().all(char::is_whitespace) =>
            {
                WikidotContent::Empty
            }
            Element::Container(container) => wikidot_content(container.elements()),
            Element::Anchor { elements, .. } | Element::Color { elements, .. } => {
                wikidot_content(elements)
            }
            Element::Footnote(_) => WikidotContent::FootnoteFirst,
            Element::LineBreak => WikidotContent::Empty,
            Element::Partial(partial) if partial.is_inline_format_control() => {
                WikidotContent::Empty
            }
            _ => WikidotContent::Other,
        };
        if content != WikidotContent::Empty {
            return content;
        }
    }
    WikidotContent::Empty
}

fn prefix_has_authored_syntax(prefix: &str) -> bool {
    let mut cursor = 0;
    while cursor < prefix.len() {
        let rest = &prefix[cursor..];
        let character = rest
            .chars()
            .next()
            .expect("cursor remains on a character boundary");
        if character.is_whitespace() {
            cursor += character.len_utf8();
            continue;
        }
        if let Some(comment) = rest.strip_prefix("[!--")
            && let Some(close) = comment.find("--]")
        {
            cursor += 4 + close + 3;
            continue;
        }
        return true;
    }
    false
}

fn find_uncommented_footnote(line: &str) -> Option<usize> {
    const FOOTNOTE: &[u8] = b"[[footnote";

    let mut cursor = 0;
    while cursor < line.len() {
        let rest = &line[cursor..];
        if let Some(comment) = rest.strip_prefix("[!--") {
            let close = comment.find("--]")?;
            cursor += 4 + close + 3;
            continue;
        }
        if rest
            .as_bytes()
            .get(..FOOTNOTE.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(FOOTNOTE))
            && rest
                .as_bytes()
                .get(FOOTNOTE.len())
                .is_some_and(|byte| byte.is_ascii_whitespace() || *byte == b']')
        {
            return Some(cursor);
        }
        cursor += rest
            .chars()
            .next()
            .expect("cursor remains on a character boundary")
            .len_utf8();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_and_whitespace_do_not_acquire_footnote_prefix_authority() {
        assert!(!prefix_has_authored_syntax(""));
        assert!(!prefix_has_authored_syntax("  [!--hidden--]\t"));
        assert!(prefix_has_authored_syntax("@@@@"));
        assert!(prefix_has_authored_syntax("****"));
        assert!(prefix_has_authored_syntax("[[span]][[/span]]"));
    }

    #[test]
    fn footnote_heads_inside_comments_do_not_end_prefix_scanning() {
        let line = "[!-- [[footnote]] --]  [[FOOTNOTE]]body[[/footnote]]";
        let start = find_uncommented_footnote(line).unwrap();
        assert_eq!(&line[start..start + "[[FOOTNOTE]]".len()], "[[FOOTNOTE]]");
        assert!(!prefix_has_authored_syntax(&line[..start]));
    }
}
