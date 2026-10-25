use super::*;

/// A page shaped like the site's: a head with the fingerprinted values, a frame, and the main
/// content.
const PAGE: &str = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <link rel="canonical" href="https://ggirelli.info/about" />
    <meta name="description" content="
Filopoetica -

About me
" />
    <title>

About me

 | Filopoetica</title>
    <meta property="og:title" content="About me" />
    <link href="/assets/css/main.css" rel="stylesheet" />
  </head>
  <body class="layout-page">
    <nav><a href="/"><img src="/favicon.ico" alt="Filopoetica"></a></nav>
    <div class="main-content container">
      <!-- Post -->
      <h1 class="posttitle">
        About me
      </h1>
      <div class="article-post"><h3 id="other-things">Other things</h3><p>Books, <a href="/dg/book-club/index">club</a>.</p></div>
    </div>
    <a class="mt-1 mb-1" href="/blog/categories#journal">journal (16)</a>
    <footer>Copyright © 2012-2026</footer>
    <script src="/assets/js/jquery.min.js"></script>
  </body>
</html>
"#;

fn changed(old: &str, new: &str) -> String {
    assert!(PAGE.contains(old), "{old}");
    PAGE.replacen(old, new, 1)
}

#[test]
fn a_page_equals_itself_whatever_its_whitespace() {
    let compact = PAGE
        .replace("\n\nAbout me\n\n | Filopoetica", "About me | Filopoetica")
        .replace(
            "<h1 class=\"posttitle\">\n        About me\n      </h1>",
            "<h1 class=\"posttitle\">About me</h1>",
        )
        .replace(
            "href=\"https://ggirelli.info/about\"",
            "href=\"https:&#x2f;&#x2f;ggirelli.info&#x2f;about\"",
        );
    assert!(check_page("about.html", PAGE, &compact));
}

#[test]
fn each_part_of_the_fingerprint_is_checked() {
    for (old, new) in [
        ("About me\n\n | Filopoetica", "About | Filopoetica"),
        (
            "https://ggirelli.info/about",
            "https://ggirelli.info/about/",
        ),
        ("Filopoetica -", "Filopoetica |"),
        ("content=\"About me\" />", "content=\"About\" />"),
        ("id=\"other-things\"", "id=\"other\""),
        ("href=\"/dg/book-club/index\"", "href=\"/dg/book-club/\""),
        ("src=\"/favicon.ico\"", "src=\"/favicon.svg\""),
        (
            "src=\"/assets/js/jquery.min.js\"",
            "src=\"/assets/js/jquery.js\"",
        ),
        ("journal (16)", "journal (15)"),
        ("2012-2026", "2012-2025"),
    ] {
        assert!(
            !check_page("about.html", PAGE, &changed(old, new)),
            "{old} -> {new} passed"
        );
    }
}

#[test]
fn the_main_content_is_compared_as_a_tree() {
    for (old, new) in [
        ("<!-- Post -->", "<!-- Article -->"),
        ("<h3 id=", "<h4 id="),
        ("class=\"posttitle\"", "class=\"posttitle p-name\""),
    ] {
        let page = changed(old, new);
        let main = |html: &str| {
            let nodes = dom::normalize_document(html);
            find(&nodes, &|node| has_class(node, "main-content")).cloned()
        };
        assert_ne!(main(PAGE), main(&page), "{old} -> {new}");
        assert!(
            !check_page("about.html", PAGE, &page),
            "{old} -> {new} passed"
        );
    }
}
