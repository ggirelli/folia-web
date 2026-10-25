//! Rendering tests on real snippets of the reference site. The expected HTML is what the site's
//! kramdown (2.5.2, GFM input, Jekyll's options) makes of the same snippet, with wiki-links
//! replaced as the site's plugin replaces them; each test names where its snippet comes from.

use super::*;

struct Garden;

impl LinkResolver for Garden {
    fn resolve(&self, target: &str) -> Option<String> {
        match target.trim().to_lowercase().as_str() {
            "rust" => Some("/dg/coding/rust".to_owned()),
            "book club" => Some("/dg/book-club/index".to_owned()),
            "dune - by frank herbert" => {
                Some("/dg/book-club/book-quotes/2016-03-29-dune".to_owned())
            }
            "doctor sleep - by stephen king" => {
                Some("/dg/book-club/book-quotes/2016-06-19-doctor-sleep".to_owned())
            }
            "don't forget your spf! 🌞😎" => Some("/dg/coding/python/uv".to_owned()),
            "cli dev in python" => Some("/dg/coding/python/cli".to_owned()),
            "python: type checking" => Some("/dg/coding/python/type_checking".to_owned()),
            _ => None,
        }
    }
}

#[test]
fn a_paragraph_keeps_its_line_breaks() {
    assert_eq!(to_html("One\ntwo.\n", None), "<p>One\ntwo.</p>\n");
}

#[test]
fn a_wiki_link_becomes_a_link_or_a_broken_link() {
    assert_eq!(
        to_html("See [[Rust]] and [[Go|golang]].\n", Some(&Garden)),
        "<p>See <a href=\"/dg/coding/rust\" class=\"internal-link\">Rust</a> and \
         <span class=\"broken-link\">golang</span>.</p>\n"
    );
}

/// 1a: `_posts/2014/2014-04-14-third-sample.md`, `_digital_garden/sayings.md`
#[test]
fn heading_ids_keep_letters_of_any_script() {
    assert_eq!(
        to_html(
            "## 2. Cos’è l’autismo?\n\n#### 在原業平 (Ariwara no Narihira)\n\n#### ομηρο\n",
            None
        ),
        "<h2 id=\"2-cosè-lautismo\">2. Cos’è l’autismo?</h2>\n\n<h4 id=\"在原業平-ariwara-no-narihira\">在原業平 (Ariwara no Narihira)</h4>\n\n<h4 id=\"ομηρο\">ομηρο</h4>\n"
    );
}

/// 1a: `_posts/2022/2022-09-25-sample-week.md` (the repeat is added)
#[test]
fn emoji_headings_lose_the_emoji_and_repeats_are_numbered() {
    assert_eq!(
        to_html(
            "## 🧅 Onions and berries 🍓\n\n## 🎼 Song of the week\n\n## 🎼 Song of the week\n",
            None
        ),
        "<h2 id=\"-onions-and-berries-\">🧅 Onions and berries 🍓</h2>\n\n<h2 id=\"-song-of-the-week\">🎼 Song of the week</h2>\n\n<h2 id=\"-song-of-the-week-1\">🎼 Song of the week</h2>\n"
    );
}

/// 1b, 1g: `_posts/2022/2022-09-03-sample-palette.md`
#[test]
fn span_ials_set_attributes_on_links_and_emphasis() {
    assert_eq!(
        to_html(
            "So, I got inspired by it and implemented a sort of interactive menu/command palette that\nis accessible on every page of this website too. You can access it by pressing\n**Ctrl+<u class=\"font-monospace imenu_trigger_key\" />**{: .font-monospace} (\n<u class=\"font-monospace imenu_trigger_key\" /> as menu) or clicking\n[here](/blog/2022/09/03/command-palette){: .imenu_manual_trigger}.\n",
            None
        ),
        "<p>So, I got inspired by it and implemented a sort of interactive menu/command palette that\nis accessible on every page of this website too. You can access it by pressing\n<strong class=\"font-monospace\">Ctrl+<u class=\"font-monospace imenu_trigger_key\"></u></strong> (\n<u class=\"font-monospace imenu_trigger_key\"></u> as menu) or clicking\n<a href=\"/blog/2022/09/03/command-palette\" class=\"imenu_manual_trigger\">here</a>.</p>\n"
    );
}

/// 1b, 1c: `pages/making-of.md`
#[test]
fn a_span_ial_may_wrap_and_shortcut_references_resolve() {
    assert_eq!(
        to_html(
            "The `ggirelli.info` domain is resolved by DNS offered by [Aruba.it]{:\ntarget=\"blank\"}, proxied through [Cloudflare]{: target=\"blank\"} to point to a\n[GitHub Pages], which services the self-hosted\n\n[Aruba.it]: https://www.aruba.it/en\n[Cloudflare]: https://www.cloudflare.com/\n[GitHub Pages]: https://pages.github.com/\n",
            None
        ),
        "<p>The <code class=\"language-plaintext highlighter-rouge\">ggirelli.info</code> domain is resolved by DNS offered by <a href=\"https://www.aruba.it/en\" target=\"blank\">Aruba.it</a>, proxied through <a href=\"https://www.cloudflare.com/\" target=\"blank\">Cloudflare</a> to point to a\n<a href=\"https://pages.github.com/\">GitHub Pages</a>, which services the self-hosted</p>\n\n"
    );
}

/// 1b: `_digital_garden/web/index.md`, `_posts/2021/2021-08-08-sample-post.md`
#[test]
fn block_ials_apply_to_the_next_block_after_a_blank_line() {
    assert_eq!(
        to_html(
            "Now, the domain contains different parts:\n\n{: .mt-1em }\n\n- A [blog], where I post in short to long format.\n\n{: .mt-1em }\n\n- A digital [garden]: my take on the _second brain_ concept. My public PKM\n\n{: .lh-sm .fs-0p5em}\n**Note:** self-replies and such are not considered webmentions and Brid.gy will not\n",
            None
        ),
        "<p>Now, the domain contains different parts:</p>\n\n<ul class=\"mt-1em\">\n  <li>A [blog], where I post in short to long format.</li>\n</ul>\n\n<ul class=\"mt-1em\">\n  <li>A digital [garden]: my take on the <em>second brain</em> concept. My public PKM</li>\n</ul>\n\n<p class=\"lh-sm fs-0p5em\"><strong>Note:</strong> self-replies and such are not considered webmentions and Brid.gy will not</p>\n"
    );
}

/// lists: `_digital_garden/tips/index.md`
#[test]
fn a_list_item_gets_a_paragraph_when_a_blank_line_follows() {
    assert_eq!(
        to_html(
            "- Americas\n\n  {: .mt-n1em }\n\n  - North\n\n    {: .mt-n1em }\n\n    - Canada\n    - Mexico\n    - USA\n\n      {: .mt-n1em }\n\n      - Bay area (CA)\n      - Boston area (MA)\n",
            None
        ),
        "<ul>\n  <li>\n    <p>Americas</p>\n\n    <ul class=\"mt-n1em\">\n      <li>\n        <p>North</p>\n\n        <ul class=\"mt-n1em\">\n          <li>Canada</li>\n          <li>Mexico</li>\n          <li>\n            <p>USA</p>\n\n            <ul class=\"mt-n1em\">\n              <li>Bay area (CA)</li>\n              <li>Boston area (MA)</li>\n            </ul>\n          </li>\n        </ul>\n      </li>\n    </ul>\n  </li>\n</ul>\n"
    );
}

/// 1d: `_posts/2014/2014-01-29-second-sample.md`
#[test]
fn quotes_and_dashes_become_typographic() {
    assert_eq!(
        to_html(
            "<small><u>**ATTENZIONE**</u>: questo detto, non voglio attaccare le scelte di\nnessuna delle due parti ma solo la condotta, secondo me \"scorretta\", di\nmanipolazione dell'informazione omettendo dati essenziali. **Ovvero che la multa\n",
            None
        ),
        "<p><small><u>**ATTENZIONE**</u>: questo detto, non voglio attaccare le scelte di\nnessuna delle due parti ma solo la condotta, secondo me “scorretta”, di\nmanipolazione dell’informazione omettendo dati essenziali. **Ovvero che la multa</small></p>\n"
    );
}

/// 1g: `_posts/2013/2013-06-09-first-sample.md`
#[test]
fn inline_html_without_a_span_content_model_keeps_its_source() {
    assert_eq!(
        to_html(
            "dal significato _così difficile_. Cosa significa corretto? Cos’è corretto, giusto,\ncos’è vero? In questo caso, è corretto ciò che è scientifico, ma <u>solo<u> perché si\nsta parlando di _corretta informazione scientifica_, non per altro.\n",
            None
        ),
        "<p>dal significato <em>così difficile</em>. Cosa significa corretto? Cos’è corretto, giusto,\ncos’è vero? In questo caso, è corretto ciò che è scientifico, ma <u>solo<u> perché si\nsta parlando di _corretta informazione scientifica_, non per altro.</u></u></p>\n"
    );
}

/// 1g: `_posts/2015/2015-12-13-sample-sketch.md`
#[test]
fn a_line_starting_with_inline_html_is_a_paragraph() {
    assert_eq!(
        to_html(
            "<a href=\"/assets/images/hotlink-ok/sketches-tissue-head.webp\" target=\"_blank\">\n    <img class=\"img-fluid\" src=\"/assets/images/hotlink-ok/sketches-tissue-head.webp\" />\n</a>\n<!-- markdownlint-enable MD033 -->\n\nAnother old sketch from 2011.\n",
            None
        ),
        "<p><a href=\"/assets/images/hotlink-ok/sketches-tissue-head.webp\" target=\"_blank\">\n    <img class=\"img-fluid\" src=\"/assets/images/hotlink-ok/sketches-tissue-head.webp\" />\n</a>\n<!-- markdownlint-enable MD033 --></p>\n\n<p>Another old sketch from 2011.</p>\n"
    );
}

/// `_digital_garden/shelf/quotes/2014-06-01-sample-quote.md`
#[test]
fn a_single_tilde_is_text() {
    assert_eq!(
        to_html("del nostro… ~.~\n", None),
        "<p>del nostro… ~.~</p>\n"
    );
}

/// 1h: `_digital_garden/home.md` (the broken link is added)
#[test]
fn a_wiki_link_with_an_ial_and_a_broken_one() {
    assert_eq!(
        to_html(
            "- 🍌 [[Book club|Did you say banana? Welcome to the library!]]{: title=\"Yes... I am reading Terry Pratchett! 😉\"} 📚\n\nSee [[No such note]].\n",
            Some(&Garden)
        ),
        "<ul>\n  <li>🍌 <a href=\"/dg/book-club/index\" class=\"internal-link\" title=\"Yes... I am reading Terry Pratchett! 😉\">Did you say banana? Welcome to the library!</a> 📚</li>\n</ul>\n\n<p>See <span class=\"broken-link\">No such note</span>.</p>\n"
    );
}

/// 1f: `_posts/2022/2022-08-27-sample-setup.md`, shortened. Rouge's tokens differ; the
/// markup around them and the code text do not.
#[test]
fn a_fenced_block_gets_rouges_markup_and_pygments_classes() {
    let html = to_html("```bash\nsudo apt install ruby-full # Ruby\n```\n", None);
    assert!(html.starts_with(
        "<div class=\"language-bash highlighter-rouge\"><div class=\"highlight\">\
         <pre class=\"highlight\"><code>"
    ));
    assert!(html.ends_with("</code></pre></div></div>\n"));
    assert!(html.contains("<span class=\"c\"># Ruby</span>"));
    let code = html
        .split_once("<code>")
        .and_then(|(_, rest)| rest.split_once("</code>"))
        .map_or("", |(code, _)| code);
    let text: String = code
        .split('<')
        .map(|part| part.split_once('>').map_or(part, |(_, text)| text))
        .collect();
    assert_eq!(text, "sudo apt install ruby-full # Ruby\n");
}

/// 1f: `_posts/2020/2020-01-26-sample-code.md`, two lines, the second shortened. Rouge names every
/// call `nf`, `self` a plain name and `in` an operator word.
#[test]
fn python_calls_and_operator_words_get_rouges_classes() {
    assert_eq!(
        to_html(
            "```python\nself.__extremes_list.append(tuple(extremes))\n\
             for x in string_range.split(\"-\")\n```\n",
            None
        ),
        "<div class=\"language-python highlighter-rouge\"><div class=\"highlight\">\
         <pre class=\"highlight\"><code><span class=\"n\">self</span><span class=\"p\">.</span>\
         <span class=\"n\">__extremes_list</span><span class=\"p\">.</span>\
         <span class=\"nf\">append</span><span class=\"p\">(</span><span class=\"nf\">tuple</span>\
         <span class=\"p\">(</span><span class=\"n\">extremes</span><span class=\"p\">))</span>\n\
         <span class=\"k\">for</span> <span class=\"n\">x</span> <span class=\"ow\">in</span> \
         <span class=\"n\">string_range</span><span class=\"p\">.</span>\
         <span class=\"nf\">split</span><span class=\"p\">(</span><span class=\"sh\">\"</span>\
         <span class=\"s\">-</span><span class=\"sh\">\"</span><span class=\"p\">)</span>\n\
         </code></pre></div></div>\n"
    );
}

/// 1f: `_digital_garden/topic/deep/tool.md`, the first two lines of its TOML block.
#[test]
fn a_toml_block_gets_rouges_classes() {
    assert_eq!(
        to_html(
            "```toml\n[aliases]\n# Quick log watch with colors\n```\n",
            None
        ),
        "<div class=\"language-toml highlighter-rouge\"><div class=\"highlight\">\
         <pre class=\"highlight\"><code><span class=\"k\">[</span><span class=\"n\">aliases</span>\
         <span class=\"k\">]</span>\n<span class=\"c\"># Quick log watch with colors</span>\n\
         </code></pre></div></div>\n"
    );
}

/// 1f: `_posts/2021/2021-08-08-sample-post.md`, the first line of its first block. Rouge gives
/// tag delimiters the tag's class, and `=` the attribute's.
#[test]
fn html_tag_delimiters_get_the_tag_class() {
    assert_eq!(
        to_html(
            "```html\n<link rel=\"webmention\" \
             href=\"https://webmention.io/username/webmention\" />\n```\n",
            None
        ),
        "<div class=\"language-html highlighter-rouge\"><div class=\"highlight\">\
         <pre class=\"highlight\"><code><span class=\"nt\">&lt;link</span> \
         <span class=\"na\">rel=</span><span class=\"s\">\"webmention\"</span> \
         <span class=\"na\">href=</span>\
         <span class=\"s\">\"https://webmention.io/username/webmention\"</span> \
         <span class=\"nt\">/&gt;</span>\n</code></pre></div></div>\n"
    );
}

/// 1e: `_posts/2017/2017-01-13-sample-tools.md`, shortened.
#[test]
fn an_excerpt_ends_at_the_marker() {
    let body = "I bought a new set of brush markers from TOUCH, the cool gray set.\n\n<!--more-->\n\nMore.\n";
    assert_eq!(
        to_html(&excerpt(body), None),
        "<p>I bought a new set of brush markers from TOUCH, the cool gray set.</p>\n\n"
    );
}

// D-003: garden excerpts resolve wiki-links as the body does. Jekyll's excerpts kept them as
// written; for a note without `<!--more-->` the excerpt is the body, so the expected HTML is
// Jekyll's own body HTML for the note (the baseline's `content/` dump).

/// 1e, 1h: `_digital_garden/shelf/reading-list.md`, whole. Jekyll's excerpt made each
/// `[[Title|text]]` item a table row.
#[test]
fn a_garden_excerpt_resolves_wiki_links_as_the_body_does() {
    let body = "This is an incomplete list.\n\n- [[Dune - by Frank Herbert|Frank Herbert - Dune]]\n- [[Doctor Sleep - by Stephen King|Stephen King - Doctor Sleep]]\n";
    let expected = "<p>This is an incomplete list.</p>\n\n<ul>\n  <li><a href=\"/dg/book-club/book-quotes/2016-03-29-dune\" class=\"internal-link\">Frank Herbert - Dune</a></li>\n  <li><a href=\"/dg/book-club/book-quotes/2016-06-19-doctor-sleep\" class=\"internal-link\">Stephen King - Doctor Sleep</a></li>\n</ul>\n";
    assert_eq!(excerpt_html(body, Some(&Garden)), expected);
    assert_eq!(to_html(body, Some(&Garden)), expected);
}

/// 1e, 1h: `_digital_garden/topic/sub/index.md`, whole: a broken link, a link whose text is
/// code, and a reference link.
#[test]
fn a_garden_excerpt_shows_no_wiki_link_markup() {
    let body = "For some instructions on how to setup `python` on your system for the first\ntime, check out the page: [[First setup: python]].\n\n- [[Don't forget your SPF! 🌞😎|`uv`]]: Python package manager by Astral!\n- How to implement [[CLI dev in Python|CLIs]] in Python.\n- [[Python: type checking|Type checking]]\n- Don't you wanna dance the [mamba]? A Python package and environment manager;\n  a C++ re-implementation of `conda` that is way faster!\n\n### Related articles\n\n- [Import times and deferring](https://ggirelli.info/blog/2025/10/21/python-import-times)\n\n[mamba]: https://mamba.readthedocs.io/en/latest/\n";
    let html = excerpt_html(body, Some(&Garden));
    assert_eq!(
        html,
        "<p>For some instructions on how to setup <code class=\"language-plaintext highlighter-rouge\">python</code> on your system for the first\ntime, check out the page: <span class=\"broken-link\">First setup: python</span>.</p>\n\n<ul>\n  <li><a href=\"/dg/coding/python/uv\" class=\"internal-link\"><code class=\"language-plaintext highlighter-rouge\">uv</code></a>: Python package manager by Astral!</li>\n  <li>How to implement <a href=\"/dg/coding/python/cli\" class=\"internal-link\">CLIs</a> in Python.</li>\n  <li><a href=\"/dg/coding/python/type_checking\" class=\"internal-link\">Type checking</a></li>\n  <li>Don’t you wanna dance the <a href=\"https://mamba.readthedocs.io/en/latest/\">mamba</a>? A Python package and environment manager;\na C++ re-implementation of <code class=\"language-plaintext highlighter-rouge\">conda</code> that is way faster!</li>\n</ul>\n\n<h3 id=\"related-articles\">Related articles</h3>\n\n<ul>\n  <li><a href=\"https://ggirelli.info/blog/2025/10/21/python-import-times\">Import times and deferring</a></li>\n</ul>\n\n"
    );
    assert!(!html.contains("[[") && !html.contains("]]"));
}

/// 1e, 1h: an excerpt that ends at the marker resolves the links before it, as the body does.
#[test]
fn an_excerpt_before_the_marker_resolves_its_wiki_links() {
    let body = "See [[Rust]].\n\n<!--more-->\n\nAnd [[Book club]].\n";
    let link = "<a href=\"/dg/coding/rust\" class=\"internal-link\">Rust</a>";
    assert_eq!(
        excerpt_html(body, Some(&Garden)),
        format!("<p>See {link}.</p>\n\n")
    );
    assert!(to_html(body, Some(&Garden)).starts_with(&format!("<p>See {link}.</p>\n")));
}

/// 1h: code keeps a wiki-link as written, in an excerpt as in the body.
#[test]
fn an_excerpt_keeps_wiki_links_in_code() {
    assert_eq!(
        excerpt_html("Write `[[Rust]]` for [[Rust]].\n", Some(&Garden)),
        "<p>Write <code class=\"language-plaintext highlighter-rouge\">[[Rust]]</code> for \
         <a href=\"/dg/coding/rust\" class=\"internal-link\">Rust</a>.</p>\n"
    );
}

/// 1b, 1d, 1e: excerpts without wiki-links are the same with or without a resolver, and as
/// before D-003 (`_digital_garden/web/index.md`,
/// `_posts/2014/2014-01-29-second-sample.md`, `_posts/2017/2017-01-13-sample-tools.md`).
#[test]
fn an_excerpt_without_wiki_links_does_not_change() {
    let cases = [
        (
            "Now, the domain contains different parts:\n\n{: .mt-1em }\n\n- A [blog], where I post.\n\n{: .lh-sm .fs-0p5em}\n**Note:** self-replies\n",
            "<p>Now, the domain contains different parts:</p>\n\n<ul class=\"mt-1em\">\n  <li>A [blog], where I post.</li>\n</ul>\n\n<p class=\"lh-sm fs-0p5em\"><strong>Note:</strong> self-replies</p>\n",
        ),
        (
            "secondo me \"scorretta\", di\nmanipolazione dell'informazione -- e altro...\n",
            "<p>secondo me “scorretta”, di\nmanipolazione dell’informazione – e altro…</p>\n",
        ),
        (
            "I bought a new set of brush markers from TOUCH, the cool gray set.\n\n<!--more-->\n\nMore.\n",
            "<p>I bought a new set of brush markers from TOUCH, the cool gray set.</p>\n\n",
        ),
    ];
    for (body, expected) in cases {
        assert_eq!(excerpt_html(body, None), expected);
        assert_eq!(excerpt_html(body, Some(&Garden)), expected);
    }
}
