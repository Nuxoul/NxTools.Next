use nxtools_cli::parse_invocation;
use std::path::PathBuf;

#[test]
fn archive_list_is_available_through_original_cli_entry() {
    let invocation = parse_invocation(
        ["archive", "list", "C:\\owned\\tiny.zip", "--json"],
        PathBuf::from("C:\\owned\\downloads"),
    );
    assert!(
        invocation.is_ok(),
        "archive list must be a real CLI command: {invocation:?}"
    );
}

#[test]
fn archive_extract_requires_destination_and_never_silently_accepts_secrets_or_download_policy() {
    for args in [
        vec!["archive", "extract", "tiny.zip"],
        vec!["archive", "list", "tiny.zip", "--dest", "out"],
        vec!["archive", "list", "tiny.zip", "--password", "private-value"],
        vec!["archive", "test", "tiny.zip", "--json", "--json"],
        vec!["archive", "test", "tiny.zip", "--codepage", "99999"],
        vec!["--connect-only", "archive", "list", "tiny.zip"],
    ] {
        let error = parse_invocation(args, PathBuf::new()).expect_err("strict archive syntax");
        assert_eq!(error.exit_code, 2);
        assert!(!error.message.contains("private-value"));
    }
}

#[test]
fn archive_extract_explicit_policy_and_download_compatibility_use_original_entry() {
    use nxtools_cli::{CliCommand, archive::ArchiveAction};
    let parsed = parse_invocation(
        [
            "archive",
            "extract",
            "tiny.zip",
            "--dest",
            "out",
            "--conflict",
            "skip",
            "--create-destination",
            "--json",
        ],
        PathBuf::new(),
    )
    .expect("archive extract");
    let CliCommand::Archive(command) = parsed.command else {
        panic!("archive route")
    };
    assert_eq!(command.action, ArchiveAction::Extract);
    assert_eq!(command.destination, Some(PathBuf::from("out")));
    assert!(command.create_destination && command.json);
    assert!(matches!(
        parse_invocation(["download", "list", "--json"], PathBuf::new())
            .expect("original download")
            .command,
        CliCommand::List { json: true, .. }
    ));
}

#[test]
fn programmatic_connect_only_cannot_start_archive_host() {
    use nxtools_cli::{CliHostPolicy, run_with_host_policy};
    let invocation = parse_invocation(["archive", "list", "tiny.zip", "--json"], PathBuf::new())
        .expect("archive command");
    let root = tempfile::tempdir().expect("owned empty executable directory");
    let error = run_with_host_policy(
        invocation.command,
        &root.path().join("NxTools.Cli.exe"),
        CliHostPolicy::ConnectOnly,
    )
    .expect_err("archive connect-only rejected");
    assert_eq!(
        error.exit_code, 2,
        "programmatic policy must reject before host lookup/start"
    );
    assert_eq!(
        std::fs::read_dir(root.path())
            .expect("unchanged directory")
            .count(),
        0
    );
}
