/*
 * tree/anchor.rs
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

use std::convert::TryFrom;
use strum_macros::IntoStaticStr;

#[derive(
    Serialize, Deserialize, IntoStaticStr, Debug, Copy, Clone, Hash, PartialEq, Eq,
)]
#[serde(rename_all = "kebab-case")]
pub enum AnchorTarget {
    /// Open the link in a new tab.
    /// HTML attribute is `_blank`.
    NewTab,

    /// Open the link in the parent frame.
    /// HTML attribute is `_parent`.
    Parent,

    /// Open the link in the top-most frame.
    /// HTML attribute is `_top`.
    Top,

    /// Open the link in the current frame.
    /// HTML attribute is `_self`.
    /// This is the default setting, so the "anchor" field does not need to be included.
    Same,
}

impl AnchorTarget {
    #[inline]
    pub fn name(self) -> &'static str {
        self.into()
    }

    #[inline]
    pub fn html_attr(self) -> &'static str {
        match self {
            AnchorTarget::NewTab => "_blank",
            AnchorTarget::Parent => "_parent",
            AnchorTarget::Top => "_top",
            AnchorTarget::Same => "_self",
        }
    }
}

impl<'a> TryFrom<&'a str> for AnchorTarget {
    type Error = ();

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        const ANCHOR_TARGET_VALUES: [(&str, &str, AnchorTarget); 4] = [
            ("blank", "_blank", AnchorTarget::NewTab),
            ("parent", "_parent", AnchorTarget::Parent),
            ("top", "_top", AnchorTarget::Top),
            ("self", "_self", AnchorTarget::Same),
        ];

        for (value1, value2, target) in &ANCHOR_TARGET_VALUES {
            if value.eq_ignore_ascii_case(value1) || value.eq_ignore_ascii_case(value2) {
                return Ok(*target);
            }
        }

        Err(())
    }
}

#[test]
fn anchor_target_html_attributes_match_standard_targets() {
    assert_eq!(AnchorTarget::NewTab.html_attr(), "_blank");
    assert_eq!(AnchorTarget::Parent.html_attr(), "_parent");
    assert_eq!(AnchorTarget::Top.html_attr(), "_top");
    assert_eq!(AnchorTarget::Same.html_attr(), "_self");
}

#[test]
fn anchor_target_parses_named_and_html_attribute_forms() {
    assert_eq!(AnchorTarget::try_from("blank"), Ok(AnchorTarget::NewTab));
    assert_eq!(AnchorTarget::try_from("_blank"), Ok(AnchorTarget::NewTab));
    assert_eq!(AnchorTarget::try_from("PARENT"), Ok(AnchorTarget::Parent));
    assert_eq!(AnchorTarget::try_from("_parent"), Ok(AnchorTarget::Parent));
    assert_eq!(AnchorTarget::try_from("top"), Ok(AnchorTarget::Top));
    assert_eq!(AnchorTarget::try_from("_top"), Ok(AnchorTarget::Top));
    assert_eq!(AnchorTarget::try_from("self"), Ok(AnchorTarget::Same));
    assert_eq!(AnchorTarget::try_from("_self"), Ok(AnchorTarget::Same));
    assert_eq!(AnchorTarget::try_from("popup"), Err(()));
}

#[test]
fn anchor_target_names_are_available_for_all_variants() {
    assert!(!AnchorTarget::NewTab.name().is_empty());
    assert!(!AnchorTarget::Parent.name().is_empty());
    assert!(!AnchorTarget::Top.name().is_empty());
    assert!(!AnchorTarget::Same.name().is_empty());
}
