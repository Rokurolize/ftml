use super::Parser;

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct WikidotState {
    div_body_depth: usize,
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
