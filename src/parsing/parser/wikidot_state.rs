use super::Parser;

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct WikidotState {
    div_body_depth: usize,
    in_note_body: bool,
    literal_triple_link_depth: u16,
    in_collapsible: bool,
    quote_boundary_closes_body: bool,
    pending_collapsible_closer: bool,
    collapsible_closed_at_deeper_quote: bool,
}

impl Parser<'_, '_> {
    #[inline]
    pub(crate) fn in_wikidot_div_body(&self) -> bool {
        self.wikidot.div_body_depth > 0
    }

    #[inline]
    pub(crate) fn enter_wikidot_div_body(&mut self) {
        self.wikidot.div_body_depth += 1;
    }

    #[inline]
    pub(crate) fn leave_wikidot_div_body(&mut self) {
        self.wikidot.div_body_depth -= 1;
    }

    #[inline]
    pub(crate) fn in_wikidot_note_body(&self) -> bool {
        self.wikidot.in_note_body
    }

    #[inline]
    pub(crate) fn enter_wikidot_note_body(&mut self) {
        debug_assert!(!self.wikidot.in_note_body);
        self.wikidot.in_note_body = true;
    }

    #[inline]
    pub(crate) fn leave_wikidot_note_body(&mut self) {
        debug_assert!(self.wikidot.in_note_body);
        self.wikidot.in_note_body = false;
    }

    #[inline]
    pub(crate) fn in_wikidot_literal_triple_link(&self) -> bool {
        self.wikidot.literal_triple_link_depth > 0
    }

    #[inline]
    pub(crate) fn enter_wikidot_literal_triple_link(&mut self) {
        self.wikidot.literal_triple_link_depth =
            self.wikidot.literal_triple_link_depth.saturating_add(1);
    }

    #[inline]
    pub(crate) fn leave_wikidot_literal_triple_link(&mut self) {
        self.wikidot.literal_triple_link_depth =
            self.wikidot.literal_triple_link_depth.saturating_sub(1);
    }

    #[inline]
    pub(crate) fn clear_wikidot_literal_triple_links(&mut self) {
        self.wikidot.literal_triple_link_depth = 0;
    }

    pub(crate) fn in_wikidot_collapsible(&self) -> bool {
        self.wikidot.in_collapsible
    }

    pub(crate) fn quote_boundary_closes_body(&self) -> bool {
        self.wikidot.quote_boundary_closes_body
    }

    pub(crate) fn pending_wikidot_collapsible_closer(&self) -> bool {
        self.wikidot.pending_collapsible_closer
    }

    pub(crate) fn wikidot_collapsible_closed_at_deeper_quote(&self) -> bool {
        self.wikidot.collapsible_closed_at_deeper_quote
    }

    pub(crate) fn set_in_wikidot_collapsible(&mut self, value: bool) {
        self.wikidot.in_collapsible = value;
    }

    pub(crate) fn set_quote_boundary_closes_body(&mut self, value: bool) {
        self.wikidot.quote_boundary_closes_body = value;
    }

    pub(crate) fn set_pending_wikidot_collapsible_closer(&mut self, value: bool) {
        self.wikidot.pending_collapsible_closer = value;
    }

    pub(crate) fn set_wikidot_collapsible_closed_at_deeper_quote(&mut self, value: bool) {
        self.wikidot.collapsible_closed_at_deeper_quote = value;
    }
}
