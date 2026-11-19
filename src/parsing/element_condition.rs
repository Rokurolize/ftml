/*
 * parsing/element_condition.rs
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

use std::borrow::Cow;
use strum_macros::IntoStaticStr;

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
pub struct ElementCondition<'t> {
    #[serde(rename = "condition")]
    pub ctype: ElementConditionType,
    pub value: Cow<'t, str>,
}

impl<'t> ElementCondition<'t> {
    fn new(ctype: ElementConditionType, value: &'t str) -> Self {
        Self {
            ctype,
            value: Cow::Borrowed(value),
        }
    }

    pub fn parse(raw_spec: &'t str) -> Vec<ElementCondition<'t>> {
        raw_spec
            .split(' ')
            .filter(|s| !s.is_empty())
            .map(Self::new_from_value)
            .collect()
    }

    fn new_from_value(value: &'t str) -> Self {
        if let Some(value) = value.strip_prefix('+') {
            return Self::new(ElementConditionType::Required, value);
        }

        if let Some(value) = value.strip_prefix('-') {
            return Self::new(ElementConditionType::Prohibited, value);
        }

        Self::new(ElementConditionType::Present, value)
    }

    pub fn parse_cow(spec: Cow<'t, str>) -> Vec<ElementCondition<'t>> {
        match spec {
            Cow::Borrowed(spec) => Self::parse(spec),
            Cow::Owned(spec) => spec
                .split(' ')
                .filter(|s| !s.is_empty())
                .map(|part| {
                    if let Some(value) = part.strip_prefix('+') {
                        return ElementCondition {
                            ctype: ElementConditionType::Required,
                            value: Cow::Owned(value.to_owned()),
                        };
                    }

                    if let Some(value) = part.strip_prefix('-') {
                        return ElementCondition {
                            ctype: ElementConditionType::Prohibited,
                            value: Cow::Owned(value.to_owned()),
                        };
                    }

                    ElementCondition {
                        ctype: ElementConditionType::Present,
                        value: Cow::Owned(part.to_owned()),
                    }
                })
                .collect(),
        }
    }

    pub fn check(conditions: &[ElementCondition], values: &[Cow<str>]) -> bool {
        Self::check_with(conditions, values, |value, condition| value == condition)
    }

    pub(crate) fn check_ignore_ascii_case(
        conditions: &[ElementCondition],
        values: &[Cow<str>],
    ) -> bool {
        Self::check_with(conditions, values, |value, condition| {
            value.eq_ignore_ascii_case(condition)
        })
    }

    fn check_with<F>(
        conditions: &[ElementCondition],
        values: &[Cow<str>],
        matches: F,
    ) -> bool
    where
        F: Fn(&str, &str) -> bool,
    {
        let mut required = true;
        let mut prohibited = true;
        let mut present = false;
        let mut had_present = false;

        for condition in conditions {
            if condition.value.is_empty() {
                return false;
            }

            let has_value = values.iter().any(|value| matches(value, &condition.value));

            match condition.ctype {
                ElementConditionType::Required => required &= has_value,
                ElementConditionType::Prohibited => prohibited &= !has_value,
                ElementConditionType::Present => {
                    present |= has_value;
                    had_present = true;
                }
            }
        }

        if !had_present {
            present = true;
        }

        required && prohibited && present
    }
}

#[derive(
    Serialize, Deserialize, IntoStaticStr, Debug, Copy, Clone, Hash, PartialEq, Eq,
)]
#[serde(rename_all = "kebab-case")]
pub enum ElementConditionType {
    Required,
    Prohibited,
    Present,
}

impl ElementConditionType {
    #[inline]
    pub fn name(self) -> &'static str {
        self.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_conditions_parse_prefixes_and_ignore_empty_segments() {
        let conditions = ElementCondition::parse(" +alpha  -beta gamma ");

        assert_eq!(
            conditions,
            vec![
                ElementCondition {
                    ctype: ElementConditionType::Required,
                    value: cow!("alpha"),
                },
                ElementCondition {
                    ctype: ElementConditionType::Prohibited,
                    value: cow!("beta"),
                },
                ElementCondition {
                    ctype: ElementConditionType::Present,
                    value: cow!("gamma"),
                },
            ]
        );
        assert!(ElementCondition::check(
            &conditions,
            &[cow!("alpha"), cow!("gamma")]
        ));
        assert!(!ElementCondition::check(
            &conditions,
            &[cow!("alpha"), cow!("beta")]
        ));
    }

    #[test]
    fn element_condition_type_names_match_variant_names() {
        assert_eq!(ElementConditionType::Required.name(), "Required");
        assert_eq!(ElementConditionType::Prohibited.name(), "Prohibited");
        assert_eq!(ElementConditionType::Present.name(), "Present");
    }
}
