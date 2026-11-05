# Triple-nested inline crossing inversion — live Wikidot rule (WIP)

Status: rule identified; implementation in progress.

## Summary

The inversion is not a pairing phenomenon. Wikidot's output is produced in two
stages, both reproducible offline:

1. **Text_Wiki inline rules**, applied as whole-source regex passes in rule
   order `Colortext, Strong, Emphasis, Underline, Strikethrough, Tt,
   Superscript, Subscript, Typography`. Each pass replaces a match with
   `\xFF<id>\xFF` start/end tokens, so later rules pair across earlier tokens
   with their own lazy (`/U`) regexes. Tokens render to a flat, possibly
   misnested tag stream.
2. **libtidy** (`WikiTransformation::purifyHtml`, `tidy_parse_string` with
   `output-xhtml`) repairs that stream. Tidy's inline stack (istack) does not
   push a second `span` while one is pushed, and a misnested inline end tag
   *swaps* istack entries (`SwitchInline`) before re-opening the remaining
   inline elements (`InlineDup1`). The swap is what inverts strong/em.

For `--a##red|xb**//--c##--//**##` the stage-1 stream is

    <s>a<C>xb<strong><em></s>c</C>—</em></strong>##        (s, C are both <span>)

and tidy yields the live output

    <s>a<C>xb<em><strong>c</strong>—</em></C>##</s>

Evidence and details below.
