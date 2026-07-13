use crate::tree::{
    AttributeMap, Container, ContainerType, Element, ListItem, PartialElement,
};
use std::collections::BTreeSet;
use std::mem;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScopeKind {
    Size,
    Span,
}

struct ActiveScope<'t> {
    kind: ScopeKind,
    ctype: ContainerType,
    attributes: AttributeMap<'t>,
}

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
    stack: &mut Vec<(ScopeKind, usize)>,
    valid: &mut BTreeSet<usize>,
) {
    for element in elements {
        match element {
            Element::Partial(PartialElement::InlineSizeOpen(_)) => {
                stack.push((ScopeKind::Size, *ordinal));
                *ordinal += 1;
            }
            Element::Partial(PartialElement::InlineSizeClose) => {
                let close = *ordinal;
                *ordinal += 1;
                if let Some(position) =
                    stack.iter().rposition(|(kind, _)| *kind == ScopeKind::Size)
                {
                    let (_, open) = stack.remove(position);
                    valid.insert(open);
                    valid.insert(close);
                }
            }
            Element::Partial(PartialElement::InlineSpanOpen(_)) => {
                stack.push((ScopeKind::Span, *ordinal));
                *ordinal += 1;
            }
            Element::Partial(PartialElement::InlineSpanClose(_)) => {
                let close = *ordinal;
                *ordinal += 1;
                if let Some(position) =
                    stack.iter().rposition(|(kind, _)| *kind == ScopeKind::Span)
                {
                    let (_, open) = stack.remove(position);
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
    active: &mut Vec<ActiveScope<'t>>,
) {
    let mut output = Vec::with_capacity(elements.len());
    let mut run = Vec::new();

    for mut element in mem::take(elements) {
        if let Element::Partial(PartialElement::InlineSizeOpen(style)) = element {
            flush_run(&mut output, &mut run, active);
            let current = *ordinal;
            *ordinal += 1;
            if valid.contains(&current) {
                let mut attributes = AttributeMap::new();
                attributes.insert("style", style);
                active.push(ActiveScope {
                    kind: ScopeKind::Size,
                    ctype: ContainerType::Size,
                    attributes,
                });
            }
            continue;
        }
        if matches!(element, Element::Partial(PartialElement::InlineSizeClose)) {
            flush_run(&mut output, &mut run, active);
            let current = *ordinal;
            *ordinal += 1;
            if valid.contains(&current) {
                remove_active(active, ScopeKind::Size);
            }
            continue;
        }
        if let Element::Partial(PartialElement::InlineSpanOpen(attributes)) = element {
            flush_run(&mut output, &mut run, active);
            let current = *ordinal;
            *ordinal += 1;
            if valid.contains(&current) {
                active.push(ActiveScope {
                    kind: ScopeKind::Span,
                    ctype: ContainerType::Span,
                    attributes,
                });
            }
            continue;
        }
        if let Element::Partial(PartialElement::InlineSpanClose(source)) = element {
            flush_run(&mut output, &mut run, active);
            let current = *ordinal;
            *ordinal += 1;
            if valid.contains(&current) {
                remove_active(active, ScopeKind::Span);
            } else {
                output.push(Element::Text(source));
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
    active: &[ActiveScope<'t>],
) {
    if run.is_empty() {
        return;
    }
    let mut wrapped = mem::take(run);
    for scope in active.iter().rev() {
        wrapped = vec![Element::Container(Container::new(
            scope.ctype,
            wrapped,
            scope.attributes.clone(),
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
    active: &mut Vec<ActiveScope<'t>>,
) -> bool {
    let mut lowered = false;
    let mut visit = |children: &mut Vec<Element<'t>>| {
        lowered = true;
        lower_sequence(children, valid, ordinal, active);
    };
    visit_children_mut(element, &mut visit);
    lowered
}

fn remove_active(active: &mut Vec<ActiveScope<'_>>, kind: ScopeKind) {
    if let Some(position) = active.iter().rposition(|scope| scope.kind == kind) {
        active.remove(position);
    }
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
        Element::Partial(partial) => match partial {
            PartialElement::ListItem(ListItem::Elements { elements, .. }) => {
                visit(elements)
            }
            PartialElement::ListItem(ListItem::SubList { element }) => {
                visit_children(element, visit)
            }
            PartialElement::TableRow(row) => {
                for cell in &row.cells {
                    visit(&cell.elements);
                }
            }
            PartialElement::TableCell(cell) => visit(&cell.elements),
            PartialElement::Tab(tab) => visit(&tab.elements),
            PartialElement::RubyText(ruby_text) => visit(&ruby_text.elements),
            PartialElement::InlineSizeOpen(_)
            | PartialElement::InlineSizeClose
            | PartialElement::InlineSpanOpen(_)
            | PartialElement::InlineSpanClose(_) => {}
        },
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
        Element::Partial(partial) => match partial {
            PartialElement::ListItem(ListItem::Elements { elements, .. }) => {
                visit(elements)
            }
            PartialElement::ListItem(ListItem::SubList { element }) => {
                visit_children_mut(element, visit)
            }
            PartialElement::TableRow(row) => {
                for cell in &mut row.cells {
                    visit(&mut cell.elements);
                }
            }
            PartialElement::TableCell(cell) => visit(&mut cell.elements),
            PartialElement::Tab(tab) => visit(&mut tab.elements),
            PartialElement::RubyText(ruby_text) => visit(&mut ruby_text.elements),
            PartialElement::InlineSizeOpen(_)
            | PartialElement::InlineSizeClose
            | PartialElement::InlineSpanOpen(_)
            | PartialElement::InlineSpanClose(_) => {}
        },
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn lowering_traverses_partial_list_items_without_paragraph_safety_checks() {
        let mut elements = vec![Element::Partial(PartialElement::ListItem(
            ListItem::Elements {
                attributes: AttributeMap::new(),
                elements: vec![
                    Element::Partial(PartialElement::InlineSizeOpen(cow!("170%"))),
                    text!("partial body"),
                    Element::Partial(PartialElement::InlineSizeClose),
                ],
            },
        ))];

        lower_wikidot_inline_size_scopes(&mut elements);

        let Element::Partial(PartialElement::ListItem(ListItem::Elements {
            elements: nested,
            ..
        })) = &elements[0]
        else {
            panic!("partial list item was not preserved: {elements:#?}");
        };
        let Element::Container(container) = &nested[0] else {
            panic!("inline size scope was not lowered: {nested:#?}");
        };
        assert_eq!(container.ctype(), ContainerType::Size);
        assert_eq!(container.elements(), &[text!("partial body")]);
    }

    #[test]
    fn legacy_parse_reports_malformed_list_cell_without_panicking() {
        // Frozen EN adoption-poster-hx contains this malformed list/cell boundary.
        let source = "[[size 170%]]heading[[/size]]\n* [[/cell]]\n";
        let tokens = crate::tokenize(source);
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);

        let (tree, errors) = crate::parse(&tokens, &page_info, &settings).into();

        assert!(!tree.elements.is_empty());
        assert!(!errors.is_empty());
        assert!(
            format!("{errors:#?}").contains("NoRulesMatch"),
            "{errors:#?}",
        );
    }
}
