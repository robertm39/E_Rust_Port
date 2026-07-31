use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const UPSTREAM_EXECUTABLE_DIRECTORIES: &[&str] = &["PROVER", "SIMPLE_APPS", "EXTERNAL"];
const EXECUTABLE_RENAMES: &[(&str, &str)] = &[
    ("eprover", "umlaut"),
    ("CSSCPA_filter", "umlaut-csscpa-filter"),
    ("e_stratpar", "umlaut-stratpar"),
    ("e_ltb_runner", "umlaut-ltb-runner"),
    ("termprops", "umlaut-termprops"),
    ("term2dag", "umlaut-term2dag"),
    ("ex_commandline", "umlaut-commandline-example"),
    ("epclextract", "umlaut-pcl-extract"),
    ("epclanalyse", "umlaut-pcl-analyse"),
    ("checkproof", "umlaut-checkproof"),
    ("epcllemma", "umlaut-pcl-lemma"),
    ("edpll", "umlaut-dpll"),
    ("eground", "umlaut-ground"),
    ("classify_problem", "umlaut-classify-problem"),
    ("tsm_classify", "umlaut-tsm-classify"),
    ("direct_examples", "umlaut-direct-examples"),
    ("e_client", "umlaut-client"),
    ("e_deduction_server", "umlaut-deduction-server"),
    ("e_server", "umlaut-server"),
    ("e_axfilter", "umlaut-axiom-filter"),
    ("enormalizer", "umlaut-normalizer"),
    ("epatternize", "umlaut-patternize"),
    ("ekb_create", "umlaut-kb-create"),
    ("ekb_delete", "umlaut-kb-delete"),
    ("ekb_insert", "umlaut-kb-insert"),
    ("ekb_ginsert", "umlaut-kb-ginsert"),
];

#[test]
fn every_upstream_entry_point_maps_to_exactly_one_umlaut_binary() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let upstream = upstream_programs(root);
    let rust = rust_programs(root, false);
    let expected_upstream = EXECUTABLE_RENAMES
        .iter()
        .map(|(old, _)| (*old).to_owned())
        .collect::<BTreeSet<_>>();
    let expected_umlaut = EXECUTABLE_RENAMES
        .iter()
        .map(|(_, new)| (*new).to_owned())
        .collect::<BTreeSet<_>>();

    assert_eq!(
        upstream, expected_upstream,
        "rename table must exactly cover the standalone E entry points"
    );
    assert_eq!(
        rust, expected_umlaut,
        "Cargo must expose exactly the canonical Umlaut executable suite"
    );
}

#[test]
fn cargo_package_and_library_identity_is_umlaut() {
    assert_eq!(env!("CARGO_PKG_NAME"), "umlaut");
}

#[test]
fn old_executable_names_are_not_cargo_targets() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let rust = rust_programs(root, false);

    for (old, new) in EXECUTABLE_RENAMES {
        assert!(!rust.contains(*old), "legacy Cargo target {old} remains");
        assert!(
            rust.contains(*new),
            "canonical Cargo target {new} is missing"
        );
    }
}

#[test]
fn opt_in_executable_is_additive_and_feature_required() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let default_programs = rust_programs(root, false);
    let all_programs = rust_programs(root, true);
    let additive = all_programs
        .difference(&default_programs)
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert_eq!(additive, ["umlaut-viras-qe"]);

    let manifest_path = root.join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", manifest_path.display()));
    let viras_block = manifest
        .split("[[bin]]")
        .find(|block| block.contains("name = \"umlaut-viras-qe\""))
        .expect("VIRAS binary block");
    assert!(
        viras_block.contains("required-features = [\"viras-qe\"]"),
        "the additive arithmetic executable must remain feature-required"
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

fn rust_programs(root: &Path, include_feature_required: bool) -> BTreeSet<String> {
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
        let feature_required = fields
            .iter()
            .any(|(key, _value)| *key == "required-features");
        if feature_required && !include_feature_required {
            continue;
        }
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
