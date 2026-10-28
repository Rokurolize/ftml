## Syntax Documentation: Modules

ftml uses the term "module" to refer any name accepted by the `[[module]]` block which has associated properties. Examples include `[[module Rate]]`, `[[module CSS]]`, and `[[module ListPages]]`.

The text before the first space or end of the block head is the "name", and designates which module should be used. This is always case-insensitive.

See [Blocks](Blocks.md) for an explanation for common concepts in Wikidot blocks, such as arguments and bodies.  This document assumes knowledge of those terms. See also [Wikidot's documentation on modules](https://www.wikidot.com/doc-modules:start), not all of which are implemented here.

## List of Modules

The table below follows essentially the same schema as for blocks in general, with a few changes. [As noted above](#blocks), all modules accept separate newlines and do not accept star or score flags. Additionally, the list of accepted names is the same as the module name (but case-insensitive).

| Module Name               | Body | AST Output           | HTML Output                               | Notes |
|---------------------------|------|----------------------|-------------------------------------------|-------|
| [Backlinks](#backlinks)   | None | `Module::Backlinks`  | `<div class="backlinks-module-box"> <ul>` | |
| [Categories](#categories) | None | `Module::Categories` | `<div class="categories-module-box">`     | |
| [CSS](#css)               | Raw  | N/A                  | `<style>`                                 | Outputs contents as CSS. Alias for `[[css]]`. |
| [Join](#join)             | None | `Module::Join`       | `<div class="join-box">`                  | |
| [ListPages](#listpages)   | Raw  | `Module::ListPages`  | N/A                                       | Delayed evaluation node; execution belongs to application code. |
| [PageTree](#pagetree)     | None | `Module::PageTree`   | `<div class="pagetree-module-box"> <ul>`  | |
| [Rate](#rate)             | None | `Module::Rate`       | `<div class="page-rate-widget-box">`      | |
| [Redirect](#redirect)     | None | `Module::Redirect`   | N/A                                       | Delayed evaluation node; HTTP redirect and notice belong to application code. |
| [TagCloud](#tagcloud)     | None | `Module::TagCloud`   | N/A                                       | Delayed evaluation node; execution belongs to application code. |

### Backlinks

Provides a list of pages which link to the page in question.

Body: None

Arguments:
* `page` &mdash; (Optional) The page to retrieve the back links to. Default: current page.

Example:

```
[[module Backlinks page="scp-173"]]
```

### Categories

Lists all categories on the site, and for each, a collapsible containing a list of each page within that category.

Body: None

Arguments:
* `includeHidden` &mdash; (Optional, Boolean) Whether hidden categories (those beginning with `_`) should be shown. Default: false.

Example:

```
[[module Categories]]
```

### CSS

Adds CSS styling that will be applied to the current page. An alias of `[[css]]`.

Body: Raw

Arguments:
* None

Example:

```
[[module CSS]]
#page-title {
    color: purple;
}
[[/module CSS]]
```

### Join

A button which permits users to join or apply to the current site.

Body: None

Arguments:
* `button` &mdash; (Optional, String) The text that should be present on the button.
* All accepted attributes.

Example:

```
[[module Join]]
```

### ListPages

Preserves a ListPages invocation for delayed evaluation by application code.

Body: Raw

Arguments:
* Any arguments are preserved in source order, including duplicate names, empty values, and unknown names.

Example:

```text
[[module ListPages category="-fragment" tag="+scp" order=""]]
%%title_linked%%
%%content%%
[[/module]]
```

### PageTree

Lists all the child pages of the page in question, including their children, in a hierarchical tree.

Body: None

Arguments:
* `root` &mdash; (Optional, Slug) The page to retrieve children for. Default: current page.
* `depth` &mdash; (Optional, Integer > 0) How deep the tree should span. Default: no limit.
* `showRoot` &mdash; (Optional, Boolean) Whether to show the root page in the tree. Default: false.

Example:

```
[[module PageTree root="scp-001" showroot="true"]]
```

### Rate

Body: None

Provides a rating module, which enables votes to be cast on a page.

Arguments:
* None

Example:
```
[[module rate]]
```

### Redirect

Performs a "301 Permanently Moved" redirection to another page or URL.

Body: None

Arguments:
* `destination` &mdash; (Optional, String) The page name or URL to redirect to. Preserved verbatim; `None` when the attribute is absent.

The HTTP redirect, trailing-slash mapping, and the no-redirect information box all need the request URL and viewer context, so they belong to application code. FTML only preserves the authored destination.

Example:

```
[[module Redirect destination="start"]]
```

### TagCloud

Creates a customizable tag cloud for tagged pages, in 2D or 3D mode.

Body: None

Arguments (all preserved verbatim in source order, including duplicate names, empty values, and unknown names):
* `mode` &mdash; (Optional, String) `3d` turns on 3D mode.
* `maxFontSize`, `minFontSize` &mdash; (Optional, String) Font sizes for the most/least common tags (units of px, pt, em or %).
* `maxColor`, `minColor` &mdash; (Optional, String) Colors for the most/least common tags (`RRR,GGG,BBB` each 0-255).
* `limit` &mdash; (Optional, String) How many tags to display.
* `target` &mdash; (Optional, String) Target page for tag links.
* `category` &mdash; (Optional, String) Limits tags to the specified page category.
* `showHidden` &mdash; (Optional, String) Shows tags starting with underscore.
* `urlAttrPrefix` &mdash; (Optional, String) Prefix for parameters in generated tag URLs.
* `skipCategoryFromUrl` &mdash; (Optional, String) Omits the category parameter from generated tag URLs.
* `width`, `height` &mdash; (Optional, String) Dimensions of the 3D tag cloud.

Tag selection, limits, style interpolation, link building, and the legacy 3D SWF wrapper all need the site's tags, so they belong to application code. FTML only preserves the authored arguments.

Example:

```text
[[module TagCloud minFontSize="1em" maxFontSize="2em" maxColor="8,8,8" minColor="200,200,228" category="news"]]
```
