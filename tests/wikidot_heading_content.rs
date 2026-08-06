use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::{Render, html::HtmlRender};
use ftml::settings::{WikitextMode, WikitextSettings};
use std::borrow::Cow;
use std::time::{Duration, Instant};

fn render(source: &str) -> String {
    let page_info = PageInfo {
        page: Cow::Borrowed("heading-content"),
        category: None,
        site: Cow::Borrowed("syntax-differential"),
        title: Cow::Borrowed("Heading content"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: Vec::new(),
        language: Cow::Borrowed("en"),
    };
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let mut source = source.to_owned();
    ftml::preprocess_for_layout(&mut source, settings.layout);
    let tokenization = ftml::tokenize(&source);
    let (tree, errors) = ftml::parse(&tokenization, &page_info, &settings).into();
    assert!(errors.is_empty(), "{source:?}: {errors:#?}");
    HtmlRender.render(&tree, &page_info, &settings).body
}

#[test]
fn wikidot_heading_content_matrix_matches_live_dom() {
    for (case_id, source, expected) in [
        (
            "scout-heading-focus-h1-empty",
            "+ ",
            "<h1 id=\"toc0\"></h1>",
        ),
        (
            "scout-heading-focus-h1-space",
            "+  ",
            "<h1 id=\"toc0\"></h1>",
        ),
        (
            "scout-heading-focus-h1-comment",
            "+ [!--hidden--]",
            "<h1 id=\"toc0\"></h1>",
        ),
        (
            "scout-heading-focus-h1-raw-empty",
            "+ @@@@",
            "<h1 id=\"toc0\"></h1>",
        ),
        (
            "scout-heading-focus-h1-bold-empty",
            "+ ****",
            "<h1 id=\"toc0\"></h1>",
        ),
        (
            "scout-heading-focus-h1-italic-empty",
            "+ ////",
            "<h1 id=\"toc0\"></h1>",
        ),
        (
            "scout-heading-focus-h1-mono-empty",
            "+ {{}}",
            "<h1 id=\"toc0\"></h1>",
        ),
        (
            "scout-heading-focus-h1-span-empty",
            "+ [[span]][[/span]]",
            "<h1 id=\"toc0\"></h1>",
        ),
        (
            "scout-heading-focus-h1-text",
            "+ A",
            "<h1 id=\"toc0\"><span>A</span></h1>",
        ),
        (
            "scout-heading-focus-h1-comment-text",
            "+ [!--hidden--]A",
            "<h1 id=\"toc0\"><span>A</span></h1>",
        ),
        (
            "scout-heading-focus-h1-raw-text",
            "+ @@A@@",
            "<h1 id=\"toc0\"><span><span style=\"white-space: pre-wrap;\">A</span></span></h1>",
        ),
        (
            "scout-heading-focus-h1-bold-text",
            "+ **A**",
            "<h1 id=\"toc0\"><span><strong>A</strong></span></h1>",
        ),
        (
            "scout-heading-focus-h1-span-text",
            "+ [[span]]A[[/span]]",
            "<h1 id=\"toc0\"><span><span>A</span></span></h1>",
        ),
        (
            "scout-heading-focus-h1-footnote-first",
            "+ [[footnote]]N[[/footnote]]",
            "<p>+<sup class=\"footnoteref\"><a id=\"footnoteref-1\" href=\"javascript:;\" class=\"footnoteref\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnote-1&#39;)\">1</a></sup></p><div class=\"footnotes-footer\"><div class=\"title\">Footnotes</div><div class=\"footnote-footer\" id=\"footnote-1\"><a href=\"javascript:;\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnoteref-1&#39;)\">1</a>. N</div></div>",
        ),
        (
            "scout-heading-focus-h1-footnote-first-then-text",
            "+ [[footnote]]N[[/footnote]]A",
            "<p>+<sup class=\"footnoteref\"><a id=\"footnoteref-1\" href=\"javascript:;\" class=\"footnoteref\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnote-1&#39;)\">1</a></sup>A</p><div class=\"footnotes-footer\"><div class=\"title\">Footnotes</div><div class=\"footnote-footer\" id=\"footnote-1\"><a href=\"javascript:;\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnoteref-1&#39;)\">1</a>. N</div></div>",
        ),
        (
            "scout-heading-focus-h1-text-then-footnote",
            "+ A[[footnote]]N[[/footnote]]",
            "<h1 id=\"toc0\"><span>A<sup class=\"footnoteref\"><a id=\"footnoteref-1\" href=\"javascript:;\" class=\"footnoteref\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnote-1&#39;)\">1</a></sup></span></h1><div class=\"footnotes-footer\"><div class=\"title\">Footnotes</div><div class=\"footnote-footer\" id=\"footnote-1\"><a href=\"javascript:;\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnoteref-1&#39;)\">1</a>. N</div></div>",
        ),
        (
            "scout-heading-focus-h2-empty",
            "++ ",
            "<h2 id=\"toc0\"></h2>",
        ),
        (
            "scout-heading-focus-h2-space",
            "++  ",
            "<h2 id=\"toc0\"></h2>",
        ),
        (
            "scout-heading-focus-h2-comment",
            "++ [!--hidden--]",
            "<h2 id=\"toc0\"></h2>",
        ),
        (
            "scout-heading-focus-h2-raw-empty",
            "++ @@@@",
            "<h2 id=\"toc0\"></h2>",
        ),
        (
            "scout-heading-focus-h2-bold-empty",
            "++ ****",
            "<h2 id=\"toc0\"></h2>",
        ),
        (
            "scout-heading-focus-h2-italic-empty",
            "++ ////",
            "<h2 id=\"toc0\"></h2>",
        ),
        (
            "scout-heading-focus-h2-mono-empty",
            "++ {{}}",
            "<h2 id=\"toc0\"></h2>",
        ),
        (
            "scout-heading-focus-h2-span-empty",
            "++ [[span]][[/span]]",
            "<h2 id=\"toc0\"></h2>",
        ),
        (
            "scout-heading-focus-h2-text",
            "++ A",
            "<h2 id=\"toc0\"><span>A</span></h2>",
        ),
        (
            "scout-heading-focus-h2-comment-text",
            "++ [!--hidden--]A",
            "<h2 id=\"toc0\"><span>A</span></h2>",
        ),
        (
            "scout-heading-focus-h2-raw-text",
            "++ @@A@@",
            "<h2 id=\"toc0\"><span><span style=\"white-space: pre-wrap;\">A</span></span></h2>",
        ),
        (
            "scout-heading-focus-h2-bold-text",
            "++ **A**",
            "<h2 id=\"toc0\"><span><strong>A</strong></span></h2>",
        ),
        (
            "scout-heading-focus-h2-span-text",
            "++ [[span]]A[[/span]]",
            "<h2 id=\"toc0\"><span><span>A</span></span></h2>",
        ),
        (
            "scout-heading-focus-h2-footnote-first",
            "++ [[footnote]]N[[/footnote]]",
            "<p>++<sup class=\"footnoteref\"><a id=\"footnoteref-1\" href=\"javascript:;\" class=\"footnoteref\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnote-1&#39;)\">1</a></sup></p><div class=\"footnotes-footer\"><div class=\"title\">Footnotes</div><div class=\"footnote-footer\" id=\"footnote-1\"><a href=\"javascript:;\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnoteref-1&#39;)\">1</a>. N</div></div>",
        ),
        (
            "scout-heading-focus-h2-footnote-first-then-text",
            "++ [[footnote]]N[[/footnote]]A",
            "<p>++<sup class=\"footnoteref\"><a id=\"footnoteref-1\" href=\"javascript:;\" class=\"footnoteref\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnote-1&#39;)\">1</a></sup>A</p><div class=\"footnotes-footer\"><div class=\"title\">Footnotes</div><div class=\"footnote-footer\" id=\"footnote-1\"><a href=\"javascript:;\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnoteref-1&#39;)\">1</a>. N</div></div>",
        ),
        (
            "scout-heading-focus-h2-text-then-footnote",
            "++ A[[footnote]]N[[/footnote]]",
            "<h2 id=\"toc0\"><span>A<sup class=\"footnoteref\"><a id=\"footnoteref-1\" href=\"javascript:;\" class=\"footnoteref\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnote-1&#39;)\">1</a></sup></span></h2><div class=\"footnotes-footer\"><div class=\"title\">Footnotes</div><div class=\"footnote-footer\" id=\"footnote-1\"><a href=\"javascript:;\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnoteref-1&#39;)\">1</a>. N</div></div>",
        ),
        (
            "scout-heading-focus-h6-empty",
            "++++++ ",
            "<h6 id=\"toc0\"></h6>",
        ),
        (
            "scout-heading-focus-h6-space",
            "++++++  ",
            "<h6 id=\"toc0\"></h6>",
        ),
        (
            "scout-heading-focus-h6-comment",
            "++++++ [!--hidden--]",
            "<h6 id=\"toc0\"></h6>",
        ),
        (
            "scout-heading-focus-h6-raw-empty",
            "++++++ @@@@",
            "<h6 id=\"toc0\"></h6>",
        ),
        (
            "scout-heading-focus-h6-bold-empty",
            "++++++ ****",
            "<h6 id=\"toc0\"></h6>",
        ),
        (
            "scout-heading-focus-h6-italic-empty",
            "++++++ ////",
            "<h6 id=\"toc0\"></h6>",
        ),
        (
            "scout-heading-focus-h6-mono-empty",
            "++++++ {{}}",
            "<h6 id=\"toc0\"></h6>",
        ),
        (
            "scout-heading-focus-h6-span-empty",
            "++++++ [[span]][[/span]]",
            "<h6 id=\"toc0\"></h6>",
        ),
        (
            "scout-heading-focus-h6-text",
            "++++++ A",
            "<h6 id=\"toc0\"><span>A</span></h6>",
        ),
        (
            "scout-heading-focus-h6-comment-text",
            "++++++ [!--hidden--]A",
            "<h6 id=\"toc0\"><span>A</span></h6>",
        ),
        (
            "scout-heading-focus-h6-raw-text",
            "++++++ @@A@@",
            "<h6 id=\"toc0\"><span><span style=\"white-space: pre-wrap;\">A</span></span></h6>",
        ),
        (
            "scout-heading-focus-h6-bold-text",
            "++++++ **A**",
            "<h6 id=\"toc0\"><span><strong>A</strong></span></h6>",
        ),
        (
            "scout-heading-focus-h6-span-text",
            "++++++ [[span]]A[[/span]]",
            "<h6 id=\"toc0\"><span><span>A</span></span></h6>",
        ),
        (
            "scout-heading-focus-h6-footnote-first",
            "++++++ [[footnote]]N[[/footnote]]",
            "<p>++++++<sup class=\"footnoteref\"><a id=\"footnoteref-1\" href=\"javascript:;\" class=\"footnoteref\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnote-1&#39;)\">1</a></sup></p><div class=\"footnotes-footer\"><div class=\"title\">Footnotes</div><div class=\"footnote-footer\" id=\"footnote-1\"><a href=\"javascript:;\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnoteref-1&#39;)\">1</a>. N</div></div>",
        ),
        (
            "scout-heading-focus-h6-footnote-first-then-text",
            "++++++ [[footnote]]N[[/footnote]]A",
            "<p>++++++<sup class=\"footnoteref\"><a id=\"footnoteref-1\" href=\"javascript:;\" class=\"footnoteref\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnote-1&#39;)\">1</a></sup>A</p><div class=\"footnotes-footer\"><div class=\"title\">Footnotes</div><div class=\"footnote-footer\" id=\"footnote-1\"><a href=\"javascript:;\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnoteref-1&#39;)\">1</a>. N</div></div>",
        ),
        (
            "scout-heading-focus-h6-text-then-footnote",
            "++++++ A[[footnote]]N[[/footnote]]",
            "<h6 id=\"toc0\"><span>A<sup class=\"footnoteref\"><a id=\"footnoteref-1\" href=\"javascript:;\" class=\"footnoteref\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnote-1&#39;)\">1</a></sup></span></h6><div class=\"footnotes-footer\"><div class=\"title\">Footnotes</div><div class=\"footnote-footer\" id=\"footnote-1\"><a href=\"javascript:;\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnoteref-1&#39;)\">1</a>. N</div></div>",
        ),
        ("scout-heading-focus-h1-notoc-empty", "+* ", ""),
        ("scout-heading-focus-h1-notoc-space", "+*  ", ""),
        (
            "scout-heading-focus-h1-notoc-comment",
            "+* [!--hidden--]",
            "",
        ),
        ("scout-heading-focus-h1-notoc-raw-empty", "+* @@@@", ""),
        ("scout-heading-focus-h1-notoc-bold-empty", "+* ****", ""),
        ("scout-heading-focus-h1-notoc-italic-empty", "+* ////", ""),
        ("scout-heading-focus-h1-notoc-mono-empty", "+* {{}}", ""),
        (
            "scout-heading-focus-h1-notoc-span-empty",
            "+* [[span]][[/span]]",
            "",
        ),
        (
            "scout-heading-focus-h1-notoc-text",
            "+* A",
            "<h1><span>A</span></h1>",
        ),
        (
            "scout-heading-focus-h1-notoc-comment-text",
            "+* [!--hidden--]A",
            "<h1><span>A</span></h1>",
        ),
        (
            "scout-heading-focus-h1-notoc-raw-text",
            "+* @@A@@",
            "<h1><span><span style=\"white-space: pre-wrap;\">A</span></span></h1>",
        ),
        (
            "scout-heading-focus-h1-notoc-bold-text",
            "+* **A**",
            "<h1><span><strong>A</strong></span></h1>",
        ),
        (
            "scout-heading-focus-h1-notoc-span-text",
            "+* [[span]]A[[/span]]",
            "<h1><span><span>A</span></span></h1>",
        ),
        (
            "scout-heading-focus-h1-notoc-footnote-first",
            "+* [[footnote]]N[[/footnote]]",
            "<p>+*<sup class=\"footnoteref\"><a id=\"footnoteref-1\" href=\"javascript:;\" class=\"footnoteref\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnote-1&#39;)\">1</a></sup></p><div class=\"footnotes-footer\"><div class=\"title\">Footnotes</div><div class=\"footnote-footer\" id=\"footnote-1\"><a href=\"javascript:;\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnoteref-1&#39;)\">1</a>. N</div></div>",
        ),
        (
            "scout-heading-focus-h1-notoc-footnote-first-then-text",
            "+* [[footnote]]N[[/footnote]]A",
            "<p>+*<sup class=\"footnoteref\"><a id=\"footnoteref-1\" href=\"javascript:;\" class=\"footnoteref\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnote-1&#39;)\">1</a></sup>A</p><div class=\"footnotes-footer\"><div class=\"title\">Footnotes</div><div class=\"footnote-footer\" id=\"footnote-1\"><a href=\"javascript:;\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnoteref-1&#39;)\">1</a>. N</div></div>",
        ),
        (
            "scout-heading-focus-h1-notoc-text-then-footnote",
            "+* A[[footnote]]N[[/footnote]]",
            "<h1><span>A<sup class=\"footnoteref\"><a id=\"footnoteref-1\" href=\"javascript:;\" class=\"footnoteref\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnote-1&#39;)\">1</a></sup></span></h1><div class=\"footnotes-footer\"><div class=\"title\">Footnotes</div><div class=\"footnote-footer\" id=\"footnote-1\"><a href=\"javascript:;\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnoteref-1&#39;)\">1</a>. N</div></div>",
        ),
        ("scout-heading-focus-h2-notoc-empty", "++* ", ""),
        ("scout-heading-focus-h2-notoc-space", "++*  ", ""),
        (
            "scout-heading-focus-h2-notoc-comment",
            "++* [!--hidden--]",
            "",
        ),
        ("scout-heading-focus-h2-notoc-raw-empty", "++* @@@@", ""),
        ("scout-heading-focus-h2-notoc-bold-empty", "++* ****", ""),
        ("scout-heading-focus-h2-notoc-italic-empty", "++* ////", ""),
        ("scout-heading-focus-h2-notoc-mono-empty", "++* {{}}", ""),
        (
            "scout-heading-focus-h2-notoc-span-empty",
            "++* [[span]][[/span]]",
            "",
        ),
        (
            "scout-heading-focus-h2-notoc-text",
            "++* A",
            "<h2><span>A</span></h2>",
        ),
        (
            "scout-heading-focus-h2-notoc-comment-text",
            "++* [!--hidden--]A",
            "<h2><span>A</span></h2>",
        ),
        (
            "scout-heading-focus-h2-notoc-raw-text",
            "++* @@A@@",
            "<h2><span><span style=\"white-space: pre-wrap;\">A</span></span></h2>",
        ),
        (
            "scout-heading-focus-h2-notoc-bold-text",
            "++* **A**",
            "<h2><span><strong>A</strong></span></h2>",
        ),
        (
            "scout-heading-focus-h2-notoc-span-text",
            "++* [[span]]A[[/span]]",
            "<h2><span><span>A</span></span></h2>",
        ),
        (
            "scout-heading-focus-h2-notoc-footnote-first",
            "++* [[footnote]]N[[/footnote]]",
            "<p>++*<sup class=\"footnoteref\"><a id=\"footnoteref-1\" href=\"javascript:;\" class=\"footnoteref\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnote-1&#39;)\">1</a></sup></p><div class=\"footnotes-footer\"><div class=\"title\">Footnotes</div><div class=\"footnote-footer\" id=\"footnote-1\"><a href=\"javascript:;\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnoteref-1&#39;)\">1</a>. N</div></div>",
        ),
        (
            "scout-heading-focus-h2-notoc-footnote-first-then-text",
            "++* [[footnote]]N[[/footnote]]A",
            "<p>++*<sup class=\"footnoteref\"><a id=\"footnoteref-1\" href=\"javascript:;\" class=\"footnoteref\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnote-1&#39;)\">1</a></sup>A</p><div class=\"footnotes-footer\"><div class=\"title\">Footnotes</div><div class=\"footnote-footer\" id=\"footnote-1\"><a href=\"javascript:;\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnoteref-1&#39;)\">1</a>. N</div></div>",
        ),
        (
            "scout-heading-focus-h2-notoc-text-then-footnote",
            "++* A[[footnote]]N[[/footnote]]",
            "<h2><span>A<sup class=\"footnoteref\"><a id=\"footnoteref-1\" href=\"javascript:;\" class=\"footnoteref\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnote-1&#39;)\">1</a></sup></span></h2><div class=\"footnotes-footer\"><div class=\"title\">Footnotes</div><div class=\"footnote-footer\" id=\"footnote-1\"><a href=\"javascript:;\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnoteref-1&#39;)\">1</a>. N</div></div>",
        ),
        ("scout-heading-focus-h6-notoc-empty", "++++++* ", ""),
        ("scout-heading-focus-h6-notoc-space", "++++++*  ", ""),
        (
            "scout-heading-focus-h6-notoc-comment",
            "++++++* [!--hidden--]",
            "",
        ),
        ("scout-heading-focus-h6-notoc-raw-empty", "++++++* @@@@", ""),
        (
            "scout-heading-focus-h6-notoc-bold-empty",
            "++++++* ****",
            "",
        ),
        (
            "scout-heading-focus-h6-notoc-italic-empty",
            "++++++* ////",
            "",
        ),
        (
            "scout-heading-focus-h6-notoc-mono-empty",
            "++++++* {{}}",
            "",
        ),
        (
            "scout-heading-focus-h6-notoc-span-empty",
            "++++++* [[span]][[/span]]",
            "",
        ),
        (
            "scout-heading-focus-h6-notoc-text",
            "++++++* A",
            "<h6><span>A</span></h6>",
        ),
        (
            "scout-heading-focus-h6-notoc-comment-text",
            "++++++* [!--hidden--]A",
            "<h6><span>A</span></h6>",
        ),
        (
            "scout-heading-focus-h6-notoc-raw-text",
            "++++++* @@A@@",
            "<h6><span><span style=\"white-space: pre-wrap;\">A</span></span></h6>",
        ),
        (
            "scout-heading-focus-h6-notoc-bold-text",
            "++++++* **A**",
            "<h6><span><strong>A</strong></span></h6>",
        ),
        (
            "scout-heading-focus-h6-notoc-span-text",
            "++++++* [[span]]A[[/span]]",
            "<h6><span><span>A</span></span></h6>",
        ),
        (
            "scout-heading-focus-h6-notoc-footnote-first",
            "++++++* [[footnote]]N[[/footnote]]",
            "<p>++++++*<sup class=\"footnoteref\"><a id=\"footnoteref-1\" href=\"javascript:;\" class=\"footnoteref\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnote-1&#39;)\">1</a></sup></p><div class=\"footnotes-footer\"><div class=\"title\">Footnotes</div><div class=\"footnote-footer\" id=\"footnote-1\"><a href=\"javascript:;\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnoteref-1&#39;)\">1</a>. N</div></div>",
        ),
        (
            "scout-heading-focus-h6-notoc-footnote-first-then-text",
            "++++++* [[footnote]]N[[/footnote]]A",
            "<p>++++++*<sup class=\"footnoteref\"><a id=\"footnoteref-1\" href=\"javascript:;\" class=\"footnoteref\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnote-1&#39;)\">1</a></sup>A</p><div class=\"footnotes-footer\"><div class=\"title\">Footnotes</div><div class=\"footnote-footer\" id=\"footnote-1\"><a href=\"javascript:;\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnoteref-1&#39;)\">1</a>. N</div></div>",
        ),
        (
            "scout-heading-focus-h6-notoc-text-then-footnote",
            "++++++* A[[footnote]]N[[/footnote]]",
            "<h6><span>A<sup class=\"footnoteref\"><a id=\"footnoteref-1\" href=\"javascript:;\" class=\"footnoteref\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnote-1&#39;)\">1</a></sup></span></h6><div class=\"footnotes-footer\"><div class=\"title\">Footnotes</div><div class=\"footnote-footer\" id=\"footnote-1\"><a href=\"javascript:;\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnoteref-1&#39;)\">1</a>. N</div></div>",
        ),
        (
            "scout-heading-focus-footnote-next-line",
            "+ A\n[[footnote]]N[[/footnote]]",
            "<h1 id=\"toc0\"><span>A<sup class=\"footnoteref\"><a id=\"footnoteref-1\" href=\"javascript:;\" class=\"footnoteref\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnote-1&#39;)\">1</a></sup></span></h1><div class=\"footnotes-footer\"><div class=\"title\">Footnotes</div><div class=\"footnote-footer\" id=\"footnote-1\"><a href=\"javascript:;\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnoteref-1&#39;)\">1</a>. N</div></div>",
        ),
        (
            "scout-heading-focus-footnote-only-next-line",
            "+ \n[[footnote]]N[[/footnote]]",
            "<p>+<sup class=\"footnoteref\"><a id=\"footnoteref-1\" href=\"javascript:;\" class=\"footnoteref\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnote-1&#39;)\">1</a></sup></p><div class=\"footnotes-footer\"><div class=\"title\">Footnotes</div><div class=\"footnote-footer\" id=\"footnote-1\"><a href=\"javascript:;\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnoteref-1&#39;)\">1</a>. N</div></div>",
        ),
        (
            "scout-heading-focus-span-multiline",
            "[[span]]\n+ Inside\n[[/span]]",
            "<h1 id=\"toc0\"><span><span>Inside</span></span></h1>",
        ),
        (
            "scout-heading-focus-span-same-line",
            "[[span]]+ Inside[[/span]]",
            "<p><span>+ Inside</span></p>",
        ),
        (
            "scout-heading-focus-nested-span",
            "+ [[span]][[span]]A[[/span]][[/span]]",
            "<h1 id=\"toc0\"><span><span><span>A</span></span></span></h1>",
        ),
        (
            "scout-heading-focus-heading-before-empty",
            "+ Real\n+ ",
            "<h1 id=\"toc0\"><span>Real</span></h1><h1 id=\"toc1\"></h1>",
        ),
        (
            "scout-heading-focus-empty-before-heading",
            "+ \n+ Real",
            "<h1 id=\"toc0\"></h1><h1 id=\"toc1\"><span>Real</span></h1>",
        ),
        (
            "scout-heading-focus-notoc-empty-before-heading",
            "+* \n+ Real",
            "<h1 id=\"toc0\"><span>Real</span></h1>",
        ),
        (
            "scout-heading-focus-heading-before-notoc-empty",
            "+ Real\n+* ",
            "<h1 id=\"toc0\"><span>Real</span></h1>",
        ),
    ] {
        assert_eq!(render(source), expected, "{case_id}: {source:?}");
    }
}

#[test]
fn many_empty_inline_owners_before_a_first_footnote_stay_bounded() {
    let empty = "[[span]][[/span]]".repeat(2_048);
    let source = format!("+ {empty}[[footnote]]N[[/footnote]]A");
    let started = Instant::now();
    let html = render(&source);
    let elapsed = started.elapsed();

    assert!(
        html.starts_with("<p>+<sup class=\"footnoteref\">"),
        "{html}"
    );
    assert!(html.contains("</sup>A</p>"), "{html}");
    assert_eq!(html.matches("footnote-footer").count(), 1, "{html}");
    assert!(
        elapsed < Duration::from_secs(3),
        "first-footnote heading eligibility took {elapsed:?}",
    );
}

#[test]
fn empty_no_toc_headings_do_not_consume_neighboring_toc_ids() {
    assert_eq!(
        render("+* \n+ Real\n+* \n++ Next"),
        concat!(
            "<h1 id=\"toc0\"><span>Real</span></h1>",
            "<h2 id=\"toc1\"><span>Next</span></h2>",
        ),
    );
}
