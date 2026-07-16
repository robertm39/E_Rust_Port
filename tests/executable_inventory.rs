use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const UPSTREAM_EXECUTABLE_DIRECTORIES: &[&str] = &["PROVER", "SIMPLE_APPS", "EXTERNAL"];

#[test]
fn every_upstream_standalone_entry_point_has_a_rust_binary() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let upstream = upstream_programs(root);
    let rust = rust_programs(root);

    assert_eq!(
        rust, upstream,
        "Cargo binary registrations must exactly cover the standalone C entry points"
    );
}

fn upstream_programs(root: &Path) -> BTreeSet<String> {
    let mut programs = BTreeSet::new();

    for directory in UPSTREAM_EXECUTABLE_DIRECTORIES {
        let path = root.join("eprover").join(directory);
        let entries = fs::read_dir(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        for entry in entries {
            let entry = entry.unwrap_or_else(|error| {
                panic!("failed to read an entry in {}: {error}", path.display())
            });
            let source_path = entry.path();
            if source_path
                .extension()
                .and_then(|extension| extension.to_str())
                != Some("c")
            {
                continue;
            }

            let source = fs::read_to_string(&source_path).unwrap_or_else(|error| {
                panic!("failed to read {}: {error}", source_path.display())
            });
            if source.contains("int main(") {
                let program = source_path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or_else(|| {
                        panic!("non-UTF-8 upstream source name: {}", source_path.display())
                    });
                assert!(
                    programs.insert(program.to_owned()),
                    "duplicate upstream program name: {program}"
                );
            }
        }
    }

    programs
}

fn rust_programs(root: &Path) -> BTreeSet<String> {
    let manifest_path = root.join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", manifest_path.display()));
    let mut programs = BTreeSet::new();

    for block in manifest.split("[[bin]]").skip(1) {
        let lines = block
            .lines()
            .map(str::trim)
            .take_while(|line| !line.starts_with('['));
        let fields = lines
            .filter_map(|line| line.split_once('='))
            .map(|(key, value)| (key.trim(), value.trim().trim_matches('"')))
            .collect::<Vec<_>>();
        let name = required_field(&fields, "name", &manifest_path);
        let source_path = root.join(required_field(&fields, "path", &manifest_path));

        assert!(
            source_path.is_file(),
            "Cargo binary {name} points to missing source {}",
            source_path.display()
        );
        let source = fs::read_to_string(&source_path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", source_path.display()));
        assert!(
            source.contains("fn main("),
            "Cargo binary {name} has no main entry point in {}",
            source_path.display()
        );
        assert!(
            programs.insert(name.to_owned()),
            "duplicate Cargo binary name: {name}"
        );
    }

    programs
}

fn required_field<'a>(fields: &'a [(&str, &str)], name: &str, manifest: &Path) -> &'a str {
    fields
        .iter()
        .find_map(|(key, value)| (*key == name).then_some(*value))
        .unwrap_or_else(|| {
            panic!(
                "missing {name} field in a [[bin]] block in {}",
                manifest.display()
            )
        })
}
