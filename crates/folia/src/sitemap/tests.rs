use super::*;

fn clock() -> SiteClock {
    SiteClock::new("Europe/Stockholm", "2026-10-25T14:43:51Z".parse().unwrap()).unwrap()
}

#[test]
fn the_sitemap_is_written_as_jekyll_sitemap_writes_it_minified() {
    let urls = [
        Url {
            url: "/blog/".to_owned(),
            lastmod: Some("2021-08-08T10:00:00Z".parse().unwrap()),
        },
        Url {
            url: "/blog/category/a&b".to_owned(),
            lastmod: None,
        },
        Url {
            url: "/me/index.html".to_owned(),
            lastmod: Some("2026-01-04T12:36:24Z".parse().unwrap()),
        },
    ];
    assert_eq!(
        xml("https://example.org", &urls, &clock()),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://www.sitemaps.org/schemas/sitemap/0.9 http://www.sitemaps.org/schemas/sitemap/0.9/sitemap.xsd\" xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n\
         <url>\n<loc>https://example.org/blog/</loc>\n<lastmod>2021-08-08T12:00:00+02:00</lastmod>\n</url>\n\
         <url>\n<loc>https://example.org/blog/category/a&amp;b</loc>\n</url>\n\
         <url>\n<loc>https://example.org/me/</loc>\n<lastmod>2026-01-04T13:36:24+01:00</lastmod>\n</url>\n\
         </urlset>\n"
    );
}

#[test]
fn an_empty_sitemap_has_its_urlset_only() {
    assert!(xml("https://example.org", &[], &clock()).ends_with(
        "sitemap.xsd\" xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n</urlset>\n"
    ));
}
