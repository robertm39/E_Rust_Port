use std::process::Command;

fn assert_fatal_prefix(binary: &str, expected_prefix: &str) {
    let output = Command::new(binary)
        .arg("--definitely-invalid-option")
        .output()
        .expect("fatal-diagnostic executable starts");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());

    let stderr = String::from_utf8(output.stderr).expect("fatal diagnostic is UTF-8");
    assert!(
        stderr.starts_with(expected_prefix),
        "expected prefix {expected_prefix:?}, got {stderr:?}"
    );
    assert_eq!(stderr.lines().count(), 1, "fatal diagnostic: {stderr:?}");
}

#[test]
fn canonical_name_entrypoint_reports_through_global_fatal_owner() {
    assert_fatal_prefix(env!("CARGO_BIN_EXE_eprover"), "eprover: ");
}

#[test]
fn argv0_entrypoint_reports_exact_invoked_name_through_global_fatal_owner() {
    let binary = env!("CARGO_BIN_EXE_termprops");

    #[cfg(windows)]
    {
        // Windows `Command` normalizes this child's argv[0] to the canonical
        // executable stem even when the image is launched through a copy.
        assert_fatal_prefix(binary, "termprops: ");
    }

    #[cfg(not(windows))]
    {
        let alias = std::env::temp_dir().join(format!(
            "termprops-diagnostic-alias-{}{}",
            std::process::id(),
            std::env::consts::EXE_SUFFIX
        ));
        std::fs::copy(binary, &alias).expect("copy termprops diagnostic alias");

        let invoked_name = alias.to_string_lossy().into_owned();

        assert_fatal_prefix(
            alias.to_str().expect("UTF-8 diagnostic alias path"),
            &format!("{invoked_name}: "),
        );
        std::fs::remove_file(alias).expect("remove termprops diagnostic alias");
    }
}
