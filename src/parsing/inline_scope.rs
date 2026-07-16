use crate::tree::{
    AttributeMap, Container, ContainerType, Element, ListItem, PartialElement,
};
use std::collections::BTreeSet;
use std::mem;

const MAX_ACTIVE_INLINE_SCOPES: usize =
    crate::parsing::parser::DEFAULT_MAX_RECURSION_DEPTH;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScopeKind {
    Size,
    Span,
}

struct PendingScope {
    ordinal: usize,
    accepted: bool,
}

struct ActiveScope<'t> {
    ctype: ContainerType,
    attributes: AttributeMap<'t>,
    previous_same_kind: Option<usize>,
    previous_active: Option<usize>,
    next_active: Option<usize>,
}

#[derive(Default)]
struct ActiveScopes<'t> {
    scopes: Vec<ActiveScope<'t>>,
    top_size: Option<usize>,
    top_span: Option<usize>,
    active_tail: Option<usize>,
}

struct ActiveScopeRevIter<'a, 't> {
    scopes: &'a [ActiveScope<'t>],
    next: Option<usize>,
}

impl<'a, 't> Iterator for ActiveScopeRevIter<'a, 't> {
    type Item = &'a ActiveScope<'t>;

    fn next(&mut self) -> Option<Self::Item> {
        let position = self.next?;
        let scope = &self.scopes[position];
        self.next = scope.previous_active;
        Some(scope)
    }
}

impl<'t> ActiveScopes<'t> {
    fn push(
        &mut self,
        kind: ScopeKind,
        ctype: ContainerType,
        attributes: AttributeMap<'t>,
    ) {
        let position = self.scopes.len();
        let previous_same_kind = self.top_mut(kind).replace(position);
        let previous_active = self.active_tail.replace(position);
        self.scopes.push(ActiveScope {
            ctype,
            attributes,
            previous_same_kind,
            previous_active,
            next_active: None,
        });
        if let Some(previous_active) = previous_active {
            self.scopes[previous_active].next_active = Some(position);
        }
    }

    fn remove(&mut self, kind: ScopeKind) {
        if let Some(position) = *self.top(kind) {
            let scope = &self.scopes[position];
            let previous_same_kind = scope.previous_same_kind;
            let previous_active = scope.previous_active;
            let next_active = scope.next_active;

            if let Some(previous_active) = previous_active {
                self.scopes[previous_active].next_active = next_active;
            }
            if let Some(next_active) = next_active {
                self.scopes[next_active].previous_active = previous_active;
            } else {
                self.active_tail = previous_active;
            }
            *self.top_mut(kind) = previous_same_kind;
        }
    }

    fn iter_active_rev(&self) -> ActiveScopeRevIter<'_, 't> {
        ActiveScopeRevIter {
            scopes: &self.scopes,
            next: self.active_tail,
        }
    }

    fn top(&self, kind: ScopeKind) -> &Option<usize> {
        match kind {
            ScopeKind::Size => &self.top_size,
            ScopeKind::Span => &self.top_span,
        }
    }

    fn top_mut(&mut self, kind: ScopeKind) -> &mut Option<usize> {
        match kind {
            ScopeKind::Size => &mut self.top_size,
            ScopeKind::Span => &mut self.top_span,
        }
    }
}

pub(crate) fn lower_wikidot_inline_size_scopes<'t>(elements: &mut Vec<Element<'t>>) {
    let mut ordinal = 0;
    // Separate stacks keep crossed size/span closure matching constant-time.
    let mut open_sizes = Vec::new();
    let mut open_spans = Vec::new();
    let mut active_count = 0;
    let mut valid = BTreeSet::new();
    collect_valid_pairs(
        elements,
        &mut ordinal,
        &mut open_sizes,
        &mut open_spans,
        &mut active_count,
        &mut valid,
    );

    ordinal = 0;
    let mut active = ActiveScopes::default();
    lower_sequence(elements, &valid, &mut ordinal, &mut active);
}

fn collect_valid_pairs(
    elements: &[Element<'_>],
    ordinal: &mut usize,
    open_sizes: &mut Vec<PendingScope>,
    open_spans: &mut Vec<PendingScope>,
    active_count: &mut usize,
    valid: &mut BTreeSet<usize>,
) {
    for element in elements {
        match element {
            Element::Partial(PartialElement::InlineSizeOpen(_)) => {
                push_pending_scope(open_sizes, ordinal, active_count);
            }
            Element::Partial(PartialElement::InlineSizeClose) => {
                let close = *ordinal;
                *ordinal += 1;
                if let Some(open) = open_sizes.pop() {
                    accept_pending_pair(open, close, active_count, valid);
                }
            }
            Element::Partial(PartialElement::InlineSpanOpen(_)) => {
                push_pending_scope(open_spans, ordinal, active_count);
            }
            Element::Partial(PartialElement::InlineSpanClose(_)) => {
                let close = *ordinal;
                *ordinal += 1;
                if let Some(open) = open_spans.pop() {
                    accept_pending_pair(open, close, active_count, valid);
                }
            }
            _ => {
                let mut visit = |children: &[Element<'_>]| {
                    collect_valid_pairs(
                        children,
                        ordinal,
                        open_sizes,
                        open_spans,
                        active_count,
                        valid,
                    )
                };
                visit_children(element, &mut visit);
            }
        }
    }
}

fn push_pending_scope(
    stack: &mut Vec<PendingScope>,
    ordinal: &mut usize,
    active_count: &mut usize,
) {
    let accepted = *active_count < MAX_ACTIVE_INLINE_SCOPES;
    if accepted {
        *active_count += 1;
    }
    stack.push(PendingScope {
        ordinal: *ordinal,
        accepted,
    });
    *ordinal += 1;
}

fn accept_pending_pair(
    open: PendingScope,
    close: usize,
    active_count: &mut usize,
    valid: &mut BTreeSet<usize>,
) {
    if open.accepted {
        *active_count -= 1;
        valid.insert(open.ordinal);
        valid.insert(close);
    }
}

fn lower_sequence<'t>(
    elements: &mut Vec<Element<'t>>,
    valid: &BTreeSet<usize>,
    ordinal: &mut usize,
    active: &mut ActiveScopes<'t>,
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
                active.push(ScopeKind::Size, ContainerType::Size, attributes);
            }
            continue;
        }
        if matches!(element, Element::Partial(PartialElement::InlineSizeClose)) {
            flush_run(&mut output, &mut run, active);
            let current = *ordinal;
            *ordinal += 1;
            if valid.contains(&current) {
                active.remove(ScopeKind::Size);
            }
            continue;
        }
        if let Element::Partial(PartialElement::InlineSpanOpen(attributes)) = element {
            flush_run(&mut output, &mut run, active);
            let current = *ordinal;
            *ordinal += 1;
            if valid.contains(&current) {
                active.push(ScopeKind::Span, ContainerType::Span, attributes);
            }
            continue;
        }
        if let Element::Partial(PartialElement::InlineSpanClose(source)) = element {
            flush_run(&mut output, &mut run, active);
            let current = *ordinal;
            *ordinal += 1;
            if valid.contains(&current) {
                active.remove(ScopeKind::Span);
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
    active: &ActiveScopes<'t>,
) {
    if run.is_empty() {
        return;
    }
    let mut wrapped = mem::take(run);
    for scope in active.iter_active_rev() {
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
    active: &mut ActiveScopes<'t>,
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

    #[test]
    fn lowering_caps_active_inline_scope_depth() {
        let mut elements = Vec::new();
        for _ in 0..(MAX_ACTIVE_INLINE_SCOPES + 1) {
            elements.push(Element::Partial(PartialElement::InlineSizeOpen(cow!(
                "font-size: larger;"
            ))));
        }
        elements.push(text!("bounded"));
        for _ in 0..(MAX_ACTIVE_INLINE_SCOPES + 1) {
            elements.push(Element::Partial(PartialElement::InlineSizeClose));
        }

        lower_wikidot_inline_size_scopes(&mut elements);

        assert_eq!(
            max_inline_scope_container_depth(&elements),
            MAX_ACTIVE_INLINE_SCOPES
        );
    }

    #[test]
    fn lowering_caps_mixed_inline_scope_depth() {
        let kinds = (0..(MAX_ACTIVE_INLINE_SCOPES + 2))
            .map(|index| {
                if index % 2 == 0 {
                    ScopeKind::Size
                } else {
                    ScopeKind::Span
                }
            })
            .collect::<Vec<_>>();
        let mut elements = Vec::new();
        for kind in &kinds {
            match kind {
                ScopeKind::Size => elements.push(Element::Partial(
                    PartialElement::InlineSizeOpen(cow!("font-size: larger;")),
                )),
                ScopeKind::Span => elements.push(Element::Partial(
                    PartialElement::InlineSpanOpen(AttributeMap::new()),
                )),
            }
        }
        elements.push(text!("mixed bounded"));
        for kind in kinds.iter().rev() {
            match kind {
                ScopeKind::Size => {
                    elements.push(Element::Partial(PartialElement::InlineSizeClose));
                }
                ScopeKind::Span => elements.push(Element::Partial(
                    PartialElement::InlineSpanClose(cow!("[[/span]]")),
                )),
            }
        }

        lower_wikidot_inline_size_scopes(&mut elements);

        assert_eq!(
            max_inline_scope_container_depth(&elements),
            MAX_ACTIVE_INLINE_SCOPES
        );
    }

    #[test]
    fn unmatched_over_limit_inline_scopes_fail_closed() {
        let mut elements = Vec::new();
        for _ in 0..(MAX_ACTIVE_INLINE_SCOPES + 1) {
            elements.push(Element::Partial(PartialElement::InlineSizeOpen(cow!(
                "font-size: larger;"
            ))));
        }
        elements.push(text!("unmatched"));

        lower_wikidot_inline_size_scopes(&mut elements);

        assert_eq!(elements, vec![text!("unmatched")]);
        assert_eq!(max_inline_scope_container_depth(&elements), 0);
    }

    fn max_inline_scope_container_depth(elements: &[Element<'_>]) -> usize {
        elements
            .iter()
            .map(|element| match element {
                Element::Container(container)
                    if matches!(
                        container.ctype(),
                        ContainerType::Size | ContainerType::Span
                    ) =>
                {
                    1 + max_inline_scope_container_depth(container.elements())
                }
                Element::Container(container) => {
                    max_inline_scope_container_depth(container.elements())
                }
                _ => 0,
            })
            .max()
            .unwrap_or(0)
    }
}
