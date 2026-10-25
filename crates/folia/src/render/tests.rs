use folia_core::content::Route;
use serde_json::json;

use super::*;

/// A throwaway template directory holding the given templates, removed when dropped.
struct Templates(PathBuf);

impl Templates {
    fn new(test: &str, files: &[(&str, &str)]) -> Self {
        let root = std::env::temp_dir().join(format!("folia-render-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (path, text) in files {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        Self(root)
    }
}

impl Drop for Templates {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn spec(url: &str, template: &str) -> PageSpec {
    PageSpec {
        route: Route::html(url).unwrap(),
        template: template.to_owned(),
        source: None,
        context: json!({"title": "About me", "modified": "2026-01-04T13:36:24+01:00"}),
    }
}

#[test]
fn a_page_renders_through_the_templates_it_extends() {
    let templates = Templates::new(
        "extends",
        &[
            (
                "base.html",
                "<title>{{ page.title }} | {{ site.title }}</title>{% block body %}{% endblock %}",
            ),
            (
                "page.html",
                "{% extends \"base.html\" %}{% block body %}{{ content | safe }}{% include \"includes/date.html\" %}{% endblock %}",
            ),
            (
                "includes/date.html",
                "<time>{{ page.modified | date(\"%b %-d, %Y\") }}</time>",
            ),
        ],
    );
    let renderer = Renderer::new(&templates.0);
    let page = spec("/about", "page.html");
    renderer.check(std::slice::from_ref(&page)).unwrap();
    let html = renderer
        .render(&page, "<p>Hi & bye</p>", &json!({"title": "Filopoetica"}))
        .unwrap();
    assert_eq!(
        html,
        "<title>About me | Filopoetica</title><p>Hi & bye</p><time>Jan 4, 2026</time>"
    );
}

#[test]
fn values_are_escaped_unless_marked_safe() {
    let templates = Templates::new("escape", &[("page.html", "{{ content }}")]);
    let html = Renderer::new(&templates.0)
        .render(&spec("/about", "page.html"), "<p>", &json!({}))
        .unwrap();
    assert_eq!(html, "&lt;p&gt;");
}

#[test]
fn an_undefined_value_is_an_error_even_in_a_test() {
    let templates = Templates::new(
        "undefined",
        &[("page.html", "{% if page.toc %}toc{% endif %}")],
    );
    let problem = Renderer::new(&templates.0)
        .render(&spec("/about", "page.html"), "", &json!({}))
        .unwrap_err();
    assert!(
        problem
            .message
            .starts_with("/about cannot be rendered with page.html: undefined value"),
        "{problem}"
    );
}

#[test]
fn every_missing_template_is_reported_at_once_with_its_pages() {
    let templates = Templates::new("missing", &[("page.html", "")]);
    let problem = Renderer::new(&templates.0)
        .check(&[
            spec("/about", "page.html"),
            spec("/now", "post.html"),
            spec("/ideas", "post.html"),
            spec("/x", "x.html"),
        ])
        .unwrap_err();
    assert_eq!(
        problem.message,
        "these templates cannot be used:\n  post.html: missing; used by /ideas, /now\n  x.html: missing; used by /x"
    );
}

#[test]
fn a_name_outside_the_template_directory_is_rejected() {
    let templates = Templates::new("names", &[("page.html", "")]);
    std::fs::write(templates.0.with_extension("html"), "outside").unwrap();
    let problem = Renderer::new(&templates.0)
        .check(&[
            spec("/a", "../folia-render-outside.html"),
            spec("/b", "/etc/passwd"),
            spec("/c", "./page.html"),
        ])
        .unwrap_err();
    let _ = std::fs::remove_file(templates.0.with_extension("html"));
    assert_eq!(
        problem.message,
        "these templates cannot be used:\
         \n  ../folia-render-outside.html: a template name must not have an empty, `.` or `..` segment; used by /a\
         \n  ./page.html: a template name must not have an empty, `.` or `..` segment; used by /c\
         \n  /etc/passwd: a template name must be a relative path; used by /b"
    );
}

#[test]
fn a_template_cannot_extend_one_outside_the_directory() {
    let templates = Templates::new(
        "escape-extends",
        &[("page.html", "{% extends \"../base.html\" %}")],
    );
    let renderer = Renderer::new(&templates.0);
    let page = spec("/about", "page.html");
    renderer.check(std::slice::from_ref(&page)).unwrap();
    let problem = renderer.render(&page, "", &json!({})).unwrap_err();
    assert!(
        problem
            .message
            .contains("../base.html: a template name must not have"),
        "{problem}"
    );
}

#[test]
fn a_template_that_does_not_compile_is_reported_by_the_check() {
    let templates = Templates::new("syntax", &[("page.html", "{% if %}")]);
    let problem = Renderer::new(&templates.0)
        .check(&[spec("/about", "page.html")])
        .unwrap_err();
    assert!(
        problem.message.contains("page.html: syntax error"),
        "{problem}"
    );
}
