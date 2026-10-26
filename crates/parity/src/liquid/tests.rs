use std::path::PathBuf;

use super::*;

/// The delimiters `find` reports in `text`, by line and column.
fn found(text: &str) -> Vec<(usize, usize, &'static str)> {
    find(text)
}

const NONE: [(usize, usize, &str); 0] = [];

#[test]
fn liquid_in_prose_is_found() {
    assert_eq!(
        found("A {% if x %}tag{% endif %}.\n"),
        [(1, 3, "{%"), (1, 16, "{%")]
    );
    assert_eq!(found("An {{ page.title }} output.\n"), [(1, 4, "{{")]);
}

/// Each finding has its own line, and lines count from the top of the file, front matter
/// included; columns count characters, not bytes.
#[test]
fn findings_have_the_file_position() {
    let text = "---\ntitle: T\n---\n\nPerché {{ a }}\n\n- {% b %}\n> {{ c }}\n";
    assert_eq!(found(text), [(5, 8, "{{"), (7, 3, "{%"), (8, 3, "{{")]);
}

/// Jekyll does not run Liquid in front matter, and Folia reads it as YAML.
#[test]
fn front_matter_is_not_checked() {
    assert_eq!(found("---\ntitle: \"{{ x }}\"\n---\nText.\n"), NONE);
}

/// Raw HTML is no exemption: Liquid runs before kramdown, inside tags, attributes, comments and
/// HTML blocks alike. So is HTML's own `code` element, which is not Markdown code.
#[test]
fn liquid_in_html_is_found() {
    let cases = [
        "<span title=\"{{ x }}\">a</span>\n",
        "Text <!-- {% comment %} --> more.\n",
        "<div>\n{{ x }}\n</div>\n",
        "<!--\n{% include a.html %}\n-->\n",
        "<code>{{ x }}</code>\n",
    ];
    for case in cases {
        assert_eq!(found(case).len(), 1, "{case:?}");
    }
}

/// A backtick without its match opens no code span, so what follows it is prose.
#[test]
fn an_unmatched_backtick_is_no_code() {
    assert_eq!(found("a ` {{ x }}\n"), [(1, 5, "{{")]);
}

/// A code span inside `u` is text in kramdown's reading, so it is not exempt.
#[test]
fn a_code_span_kramdown_keeps_as_text_is_not_code() {
    assert_eq!(found("<u>`{{ x }}`</u>\n"), [(1, 5, "{{")]);
    assert_eq!(found("<em>`{{ x }}`</em>\n"), NONE);
}

#[test]
fn liquid_in_code_is_allowed() {
    let cases = [
        "Write `{{ page.title }}` and ``{% if %}``.\n",
        "```liquid\n{% for post in site.posts %}\n  {{ post.title }}\n{% endfor %}\n```\n",
        "~~~\n{{ x }}\n~~~\n",
        "    {% raw %}{{ x }}{% endraw %}\n",
        "- item\n\n  ```\n  {{ x }}\n  ```\n",
        "> `{{ x }}`\n",
        "```\n{{ unclosed fence }}",
    ];
    for case in cases {
        assert_eq!(found(case), NONE, "{case:?}");
    }
}

/// The source is read as Liquid reads it, so parser events do not matter. `\{{` is an escaped
/// brace and a brace to Markdown, two events, but `{{` to Liquid; `{\{` is `{{` to Markdown, but
/// no delimiter to Liquid, and both show it as written.
#[test]
fn escapes_are_read_as_liquid_reads_them() {
    assert_eq!(found("A \\{{ x }} here.\n"), [(1, 4, "{{")]);
    assert_eq!(found("A \\{% x %} here.\n"), [(1, 4, "{%")]);
    assert_eq!(found("A {\\{ x }} or \\{\\% here.\n"), NONE);
}

/// A delimiter one emphasis marker splits is no delimiter; one in emphasis is.
#[test]
fn delimiters_across_markdown_syntax() {
    assert_eq!(found("*{*{ x }}\n"), NONE);
    assert_eq!(found("**{{ x }}**\n"), [(1, 3, "{{")]);
    assert_eq!(found("[{{ x }}](/a)\n"), [(1, 2, "{{")]);
}

/// Braces that open no Liquid, and the site's IALs, are fine.
#[test]
fn other_braces_are_allowed() {
    let cases = [
        "",
        "Plain text.\n",
        "A set { a, b } and }} and %} alone.\n",
        "{: .mt-1em }\n- item\n",
        "*emphasis*{: .note}\n",
        "{ {spaced} }\n",
    ];
    for case in cases {
        assert_eq!(found(case), NONE, "{case:?}");
    }
}

/// `{{{` and `{{%` are each one delimiter, as Liquid reads them.
#[test]
fn a_run_of_braces_is_one_finding() {
    assert_eq!(found("{{{ x }}}\n"), [(1, 1, "{{")]);
    assert_eq!(found("{{% x %}\n"), [(1, 1, "{{")]);
}

/// A throwaway site directory holding the given files, removed when dropped.
struct Site(PathBuf);

impl Site {
    fn new(test: &str, files: &[(&str, &str)]) -> Self {
        let root =
            std::env::temp_dir().join(format!("parity-liquid-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (path, text) in files {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        Self(root)
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const CONFIG: &str = r#"
time-zone = "Europe/Stockholm"
exclude = ["README.md", "docs"]

[roots]
blog = "_posts"
micro = "_micro"
garden = "_digital_garden"

[site]
url = "https://example.com"
title = "Example"
description = "A site."
"#;

const LIQUID: &str = "---\ntitle: T\n---\n{{ x }}\n";

/// Only Markdown documents Folia discovers are checked, in order of their path.
#[test]
fn the_site_s_markdown_documents_are_checked() {
    let site = Site::new(
        "discovery",
        &[
            ("_posts/2021/2021-08-08-a.md", LIQUID),
            ("_posts/2021/2021-08-09-b.md", "---\n---\nClean.\n"),
            ("_digital_garden/note.md", LIQUID),
            ("_micro/2021-08-08-m.markdown", LIQUID),
            ("about.md", "---\n---\n`{{ code }}`\n"),
            // Not documents of the site, or not Markdown.
            ("_posts/notes.md", LIQUID),
            ("_drafts/2021-08-08-draft.md", LIQUID),
            ("_folia/templates/page.md", LIQUID),
            ("README.md", LIQUID),
            ("docs/guide.md", LIQUID),
            ("plain.md", "{{ no front matter, so copied as it is }}\n"),
            ("page.html", LIQUID),
        ],
    );
    let config: Config = toml::from_str(CONFIG).unwrap();
    let (checked, findings) = check(&site.0, &config).unwrap();
    assert_eq!(checked, 5);
    let at = |source: &str| Finding {
        source: source.to_owned(),
        line: 4,
        column: 1,
        delimiter: "{{",
    };
    assert_eq!(
        findings,
        [
            at("_digital_garden/note.md"),
            at("_micro/2021-08-08-m.markdown"),
            at("_posts/2021/2021-08-08-a.md"),
        ]
    );
}
