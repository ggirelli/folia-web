//! The configuration reference (`docs/configuration.md`) names only keys Folia reads: its full
//! example is parsed by Folia's own configuration reader, which refuses unknown keys, and the
//! reference mentions every key of a configuration that sets all of them.
//!
//! ```sh
//! cargo test -p folia --test config_reference
//! ```

use std::path::{Path, PathBuf};

use folia::config::Config;

fn reference() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/configuration.md");
    std::fs::read_to_string(path).unwrap()
}

fn full_example(doc: &str) -> &str {
    let start = doc
        .split("```toml\n# full-example\n")
        .nth(1)
        .expect("the reference has a full example");
    start.split("```").next().unwrap()
}

#[test]
fn the_full_example_is_a_valid_configuration() {
    let doc = reference();
    let dir = std::env::temp_dir().join(format!("folia-config-ref-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path: PathBuf = dir.join("folia.toml");
    std::fs::write(&path, full_example(&doc)).unwrap();
    let config = Config::load(&path).unwrap_or_else(|e| panic!("{e}"));
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(config.time_zone, "Europe/Stockholm");
    assert!(config.slice.is_some() && config.feeds.is_some() && config.search.is_some());
}

#[test]
fn every_key_of_the_full_example_is_described_in_the_reference() {
    let doc = reference();
    let table: toml::Table = toml::from_str(full_example(&doc)).unwrap();
    let mut keys = Vec::new();
    collect(&table, &mut keys);
    assert_ne!(keys.len(), 0);
    for key in keys {
        // A key is described when the reference writes it in code quotes, or as a table's header.
        assert!(
            doc.contains(&format!("`{key}`"))
                || doc.contains(&format!("[{key}]"))
                || doc.contains(&format!("[{key}.")),
            "the reference does not describe `{key}`"
        );
    }
}

/// The key names of a table, depth first, except the free names (the authors' keys).
fn collect(table: &toml::Table, keys: &mut Vec<String>) {
    for (key, value) in table {
        if !matches!(key.as_str(), "ada") {
            keys.push(key.clone());
        }
        match value {
            toml::Value::Table(inner) => collect(inner, keys),
            toml::Value::Array(items) => {
                for item in items {
                    if let toml::Value::Table(inner) = item {
                        collect(inner, keys);
                    }
                }
            }
            _ => {}
        }
    }
}
