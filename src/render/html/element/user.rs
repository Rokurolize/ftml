/*
 * render/html/element/user.rs
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
use crate::url::normalize_href;
use std::borrow::Cow;

pub fn render_user(ctx: &mut HtmlContext, name: &str, show_avatar: bool) {
    debug!("Rendering user block (name '{name}', show-avatar {show_avatar})");

    match ctx.layout() {
        Layout::Wikidot => render_user_wikidot(ctx, name, show_avatar),
        Layout::Wikijump => render_user_wikijump(ctx, name, show_avatar),
    }
}

fn render_user_wikidot(ctx: &mut HtmlContext, name: &str, show_avatar: bool) {
    let handle = ctx.handle();

    match handle.get_user_info(name) {
        Some(user_info) => {
            let user_profile_url = normalize_href(&user_info.user_profile_url, None);
            let user_avatar_src = normalize_user_avatar_src(&user_info.user_avatar_data);
            let printuser_class = if show_avatar {
                "printuser avatarhover"
            } else {
                "printuser"
            };

            let wikidot_onclick = format!(
                "WIKIDOT.page.listeners.userInfo({}); return false;",
                user_info.user_id,
            );

            ctx.html()
                .span()
                .attr(attr!("class" => printuser_class))
                .inner(|ctx| {
                    if show_avatar {
                        // Image is wrapped in its own <a>
                        ctx.html()
                            .a()
                            .attr(attr!(
                                "href" => &user_profile_url,
                                "onclick" => &wikidot_onclick,
                            ))
                            .inner(|ctx| {
                                ctx.html()
                                    .img()
                                    .attr(attr!(
                                        "class" => "small",
                                        "src" => &user_avatar_src,
                                        "alt" => name,
                                        "style" => handle.get_karma_style(user_info.user_karma),
                                    ));
                            });
                    }

                    // Now, the username (text) with its <a>
                    ctx.html()
                        .a()
                        .attr(attr!(
                            "href" => &user_profile_url,
                            "onclick" => &wikidot_onclick,
                        ))
                        .contents(name);
                });
        }
        None => {
            let (message_pre, message_post) = {
                let page_info = ctx.info();
                let language = &page_info.language;
                let message_pre = handle.get_message(language, "user-missing-pre");
                let message_post = handle.get_message(language, "user-missing-post");
                (message_pre, message_post)
            };

            ctx.push_escaped(message_pre);

            ctx.html()
                .span()
                .attr(attr!("class" => "error-inline"))
                .inner(|ctx| {
                    // TODO localization
                    ctx.html().em().contents(name);
                });

            ctx.push_escaped(message_post);
        }
    }
}

fn render_user_wikijump(ctx: &mut HtmlContext, name: &str, show_avatar: bool) {
    ctx.html()
        .span()
        .attr(attr!("class" => "wj-user-info"))
        .inner(|ctx| match ctx.handle().get_user_info(name) {
            Some(info) => {
                let user_profile_url = normalize_href(&info.user_profile_url, None);
                let user_avatar_src = normalize_user_avatar_src(&info.user_avatar_data);
                trace!(
                    "Got user information (user id {}, name {})",
                    info.user_id,
                    info.user_name.as_ref(),
                );

                ctx.html()
                    .a()
                    .attr(attr!(
                        "class" => "wj-user-info-link",
                        "href" => &user_profile_url,
                    ))
                    .inner(|ctx| {
                        if show_avatar {
                            ctx.html()
                                .span()
                                .attr(attr!(
                                    "class" => "wj-karma",
                                    "data-karma" => &info.user_karma.to_string(),
                                ))
                                .inner(|ctx| {
                                    ctx.html().sprite("wj-karma");
                                });

                            ctx.html().img().attr(attr!(
                                "class" => "wj-user-info-avatar",
                                "src" => &user_avatar_src,
                            ));
                        }

                        ctx.html()
                            .span()
                            .attr(attr!("class" => "wj-user-info-name"))
                            .contents(&info.user_name);
                    });
            }
            None => {
                trace!("No such user found");

                ctx.html()
                    .span()
                    .attr(attr!("class" => "wj-error-inline"))
                    .inner(|ctx| {
                        if show_avatar {
                            // Karma SVG
                            ctx.html()
                                .span()
                                .attr(attr!(
                                    "class" => "wj-karma",
                                    "data-karma" => "0",
                                ))
                                .inner(|ctx| {
                                    ctx.html().sprite("wj-karma");
                                });

                            ctx.html().img().attr(attr!(
                                "class" => "wj-user-info-avatar",
                                "src" => "/files--static/media/bad-avatar.png",
                            ));
                        }

                        ctx.html()
                            .span()
                            .attr(attr!("class" => "wj-user-info-name"))
                            .contents(name);
                    });
            }
        });
}

fn normalize_user_avatar_src(src: &str) -> Cow<'_, str> {
    if is_safe_image_data_uri(src) {
        Cow::Borrowed(src)
    } else {
        normalize_href(src, None)
    }
}

fn is_safe_image_data_uri(src: &str) -> bool {
    const SAFE_DATA_PREFIXES: &[&str] = &[
        "data:image/png;base64,",
        "data:image/jpeg;base64,",
        "data:image/jpg;base64,",
        "data:image/gif;base64,",
        "data:image/webp;base64,",
        "data:image/bmp;base64,",
        "data:image/x-icon;base64,",
        "data:image/vnd.microsoft.icon;base64,",
    ];

    let Some(prefix) = SAFE_DATA_PREFIXES.iter().find(|prefix| {
        src.get(..prefix.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
    }) else {
        return false;
    };

    let data = &src[prefix.len()..];
    !data.is_empty()
        && data.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'+' | b'/' | b'=' | b'\r' | b'\n')
        })
}

#[cfg(test)]
mod tests {
    use super::{is_safe_image_data_uri, normalize_user_avatar_src};
    use crate::url::normalize_href;

    #[test]
    fn user_supplied_profile_and_avatar_urls_are_normalized() {
        assert_eq!(
            normalize_href("javascript:alert(1)", None).as_ref(),
            "#invalid-url",
        );
        assert_eq!(
            normalize_user_avatar_src("data:text/html,<script>alert(1)</script>")
                .as_ref(),
            "#invalid-url",
        );
        assert_eq!(
            normalize_user_avatar_src("data:image/svg+xml,<svg onload=alert(1)>")
                .as_ref(),
            "#invalid-url",
        );
        assert_eq!(
            normalize_user_avatar_src("data:image/png;base64,aGVsbG8=").as_ref(),
            "data:image/png;base64,aGVsbG8=",
        );

        assert!(is_safe_image_data_uri(
            "data:image/webp;base64,AAAA\r\nBBBB=="
        ));
        assert!(!is_safe_image_data_uri("data:image/png;base64,<>"));
    }
}
