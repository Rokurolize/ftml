use crate::tree::{
    AttributeMap, Container, ContainerType, Element, ListItem, PartialElement,
};
use std::borrow::Cow;
use std::collections::BTreeSet;
use std::mem;

pub(crate) fn lower_wikidot_inline_size_scopes<'t>(elements: &mut Vec<Element<'t>>) {
    let mut ordinal = 0;
    let mut stack = Vec::new();
    let mut valid = BTreeSet::new();
    collect_valid_pairs(elements, &mut ordinal, &mut stack, &mut valid);

    ordinal = 0;
    let mut active = Vec::new();
    lower_sequence(elements, &valid, &mut ordinal, &mut active);
}

fn collect_valid_pairs(
    elements: &[Element<'_>],
    ordinal: &mut usize,
    stack: &mut Vec<usize>,
    valid: &mut BTreeSet<usize>,
) {
    for element in elements {
        match element {
            Element::Partial(PartialElement::InlineSizeOpen(_)) => {
                stack.push(*ordinal);
                *ordinal += 1;
            }
            Element::Partial(PartialElement::InlineSizeClose) => {
                let close = *ordinal;
                *ordinal += 1;
                if let Some(open) = stack.pop() {
                    valid.insert(open);
                    valid.insert(close);
                }
            }
            _ => {
                let mut visit = |children: &[Element<'_>]| {
                    collect_valid_pairs(children, ordinal, stack, valid)
                };
                visit_children(element, &mut visit);
            }
        }
    }
}

fn lower_sequence<'t>(
    elements: &mut Vec<Element<'t>>,
    valid: &BTreeSet<usize>,
    ordinal: &mut usize,
    active: &mut Vec<Cow<'t, str>>,
) {
    let mut output = Vec::with_capacity(elements.len());
    let mut run = Vec::new();

    for mut element in mem::take(elements) {
        if let Element::Partial(PartialElement::InlineSizeOpen(style)) = element {
            flush_run(&mut output, &mut run, active);
            let current = *ordinal;
            *ordinal += 1;
            if valid.contains(&current) {
                active.push(style);
            }
            continue;
        }
        if matches!(element, Element::Partial(PartialElement::InlineSizeClose)) {
            flush_run(&mut output, &mut run, active);
            let current = *ordinal;
            *ordinal += 1;
            if valid.contains(&current) {
                active.pop();
            }
            continue;
        }
        if lower_children(&mut element, valid, ordinal, active) {
            flush_run(&mut output, &mut run, active);
            if !is_empty_paragraph(&element) {
                output.push(element);
            }
        } else if element.paragraph_safe() {
            run.push(element);
        } else {
            flush_run(&mut output, &mut run, active);
            output.push(element);
        }
    }
    flush_run(&mut output, &mut run, active);
    *elements = output;
}

fn flush_run<'t>(
    output: &mut Vec<Element<'t>>,
    run: &mut Vec<Element<'t>>,
    active: &[Cow<'t, str>],
) {
    if run.is_empty() {
        return;
    }
    let mut wrapped = mem::take(run);
    for style in active.iter().rev() {
        let mut attributes = AttributeMap::new();
        attributes.insert("style", style.clone());
        wrapped = vec![Element::Container(Container::new(
            ContainerType::Size,
            wrapped,
            attributes,
        ))];
    }
    output.extend(wrapped);
}

fn is_empty_paragraph(element: &Element<'_>) -> bool {
    matches!(
        element,
        Element::Container(container)
            if container.ctype() == ContainerType::Paragraph && container.elements().is_empty()
    )
}

fn lower_children<'t>(
    element: &mut Element<'t>,
    valid: &BTreeSet<usize>,
    ordinal: &mut usize,
    active: &mut Vec<Cow<'t, str>>,
) -> bool {
    let mut lowered = false;
    let mut visit = |children: &mut Vec<Element<'t>>| {
        lowered = true;
        lower_sequence(children, valid, ordinal, active);
    };
    visit_children_mut(element, &mut visit);
    lowered
}

fn visit_children<'t>(element: &Element<'t>, visit: &mut dyn FnMut(&[Element<'t>])) {
    match element {
        Element::Container(container) => visit(container.elements()),
        Element::Table(table) => {
            for row in &table.rows {
                for cell in &row.cells {
                    visit(&cell.elements);
                }
            }
        }
        Element::TabView(tabs) => {
            for tab in tabs {
                visit(&tab.elements);
            }
        }
        Element::Anchor { elements, .. }
        | Element::Collapsible { elements, .. }
        | Element::Color { elements, .. }
        | Element::Include { elements, .. } => visit(elements),
        Element::List { items, .. } => {
            for item in items {
                match item {
                    ListItem::Elements { elements, .. } => visit(elements),
                    ListItem::SubList { element } => visit_children(element, visit),
                }
            }
        }
        Element::DefinitionList(items) => {
            for item in items {
                visit(&item.key_elements);
                visit(&item.value_elements);
            }
        }
        _ => {}
    }
}

fn visit_children_mut<'t>(
    element: &mut Element<'t>,
    visit: &mut dyn FnMut(&mut Vec<Element<'t>>),
) {
    match element {
        Element::Container(container) => visit(container.elements_mut()),
        Element::Table(table) => {
            for row in &mut table.rows {
                for cell in &mut row.cells {
                    visit(&mut cell.elements);
                }
            }
        }
        Element::TabView(tabs) => {
            for tab in tabs {
                visit(&mut tab.elements);
            }
        }
        Element::Anchor { elements, .. }
        | Element::Collapsible { elements, .. }
        | Element::Color { elements, .. }
        | Element::Include { elements, .. } => visit(elements),
        Element::List { items, .. } => {
            for item in items {
                match item {
                    ListItem::Elements { elements, .. } => visit(elements),
                    ListItem::SubList { element } => visit_children_mut(element, visit),
                }
            }
        }
        Element::DefinitionList(items) => {
            for item in items {
                visit(&mut item.key_elements);
                visit(&mut item.value_elements);
            }
        }
        _ => {}
    }
}
