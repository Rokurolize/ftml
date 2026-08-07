/*
 * parsing/footnote.rs
 *
 * ftml - Library to parse Wikidot text
 * Copyright (C) 2019-2026 Wikijump Team
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 */

use crate::tree::{AttributeMap, Container, ContainerType, Element};
use std::borrow::Cow;
use std::vec::IntoIter;

pub(crate) fn normalize_wikidot_blocks(
    elements: &mut Vec<Element<'_>>,
    footnotes_empty: bool,
) {
    let root = std::mem::take(elements);
    let mut stack = vec![ElementFrame::new(None, root)];

    while let Some(frame) = stack.last_mut() {
        if let Some(element) = frame.input.next() {
            let Element::Container(mut container) = element else {
                frame.output.push(element);
                continue;
            };

            if container.ctype() == ContainerType::Paragraph {
                normalize_paragraph(&mut frame.output, container, footnotes_empty);
                continue;
            }

            let children = std::mem::take(container.elements_mut());
            stack.push(ElementFrame::new(Some(container), children));
            continue;
        }

        let ElementFrame { parent, output, .. } =
            stack.pop().expect("normalization stack was not empty");
        let Some(mut parent) = parent else {
            *elements = output;
            break;
        };
        *parent.elements_mut() = output;
        stack
            .last_mut()
            .expect("nested normalization frame has a parent")
            .output
            .push(Element::Container(parent));
    }
}

struct ElementFrame<'t> {
    parent: Option<Container<'t>>,
    input: IntoIter<Element<'t>>,
    output: Vec<Element<'t>>,
}

impl<'t> ElementFrame<'t> {
    fn new(parent: Option<Container<'t>>, input: Vec<Element<'t>>) -> Self {
        let capacity = input.len();
        Self {
            parent,
            input: input.into_iter(),
            output: Vec::with_capacity(capacity),
        }
    }
}

fn normalize_paragraph<'t>(
    output: &mut Vec<Element<'t>>,
    mut container: Container<'t>,
    footnotes_empty: bool,
) {
    if footnotes_empty {
        remove_empty_inline_blocks(container.elements_mut());
        output.push(Element::Container(container));
        return;
    }

    if !container
        .elements()
        .iter()
        .any(|element| matches!(element, Element::FootnoteBlock { .. }))
    {
        output.push(Element::Container(container));
        return;
    }

    let attributes = container.attributes().clone();
    let children: Vec<Element<'t>> = container.into();
    let mut paragraph = Vec::new();
    for child in children {
        if matches!(child, Element::FootnoteBlock { .. }) {
            push_paragraph(output, &mut paragraph, &attributes);
            output.push(child);
        } else {
            paragraph.push(child);
        }
    }
    push_paragraph(output, &mut paragraph, &attributes);
}

fn push_paragraph<'t>(
    output: &mut Vec<Element<'t>>,
    paragraph: &mut Vec<Element<'t>>,
    attributes: &AttributeMap<'t>,
) {
    if paragraph.is_empty() {
        return;
    }
    output.push(Element::Container(Container::new(
        ContainerType::Paragraph,
        std::mem::take(paragraph),
        attributes.clone(),
    )));
}

fn remove_empty_inline_blocks(elements: &mut Vec<Element<'_>>) {
    let mut output = Vec::with_capacity(elements.len());
    let mut footnote_after_hyphen = false;

    for mut element in elements.drain(..) {
        if matches!(element, Element::FootnoteBlock { .. }) {
            footnote_after_hyphen =
                matches!(output.last(), Some(Element::Text(text)) if text.ends_with('-'));
            continue;
        }

        if footnote_after_hyphen {
            let rest = match &element {
                Element::Text(text) => text.strip_prefix('-').map(str::to_owned),
                _ => None,
            };
            if let Some(rest) = rest {
                let Some(Element::Text(text)) = output.last_mut() else {
                    unreachable!("last element was checked above");
                };
                let mut value = text.to_string();
                value.pop();
                value.push('\u{2014}');
                *text = Cow::Owned(value);
                element = Element::Text(Cow::Owned(rest));
            }
            footnote_after_hyphen = false;
        }
        output.push(element);
    }

    *elements = output;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_block_normalization_handles_deep_container_trees_iteratively() {
        const DEPTH: usize = 2_048;

        let paragraph = Container::new(
            ContainerType::Paragraph,
            vec![
                Element::Text(Cow::Borrowed("before")),
                Element::FootnoteBlock {
                    title: None,
                    hide: false,
                },
                Element::Text(Cow::Borrowed("after")),
            ],
            AttributeMap::new(),
        );
        let mut elements = vec![Element::Container(paragraph)];
        for _ in 0..DEPTH {
            elements = vec![Element::Container(Container::new(
                ContainerType::Div,
                elements,
                AttributeMap::new(),
            ))];
        }

        normalize_wikidot_blocks(&mut elements, false);

        for _ in 0..DEPTH {
            let Element::Container(container) = elements.pop().expect("nested div")
            else {
                panic!("expected nested div");
            };
            assert_eq!(container.ctype(), ContainerType::Div);
            elements = container.into();
        }
        assert_eq!(elements.len(), 3);
        assert!(matches!(elements[0], Element::Container(_)));
        assert!(matches!(elements[1], Element::FootnoteBlock { .. }));
        assert!(matches!(elements[2], Element::Container(_)));
    }
}
