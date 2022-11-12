use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

type EmptyResult = Result<(), Box<dyn std::error::Error>>;

static KASH_PATH: &str = "tests/inputs/kash.ksm";

fn test_with_args(
    args: Vec<&'static str>,
    expected_output: &'static str,
    should_fail: bool,
) -> EmptyResult {
    let mut cmd = Command::cargo_bin("kdump")?;

    for arg in args {
        cmd.arg(arg);
    }

    if should_fail {
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains(expected_output));
    } else {
        cmd.assert()
            .success()
            .stdout(predicate::str::contains(expected_output));
    }

    Ok(())
}

#[test]
fn file_doesnt_exist() -> EmptyResult {
    test_with_args(
        vec!["does-not-exist-and-should-not-be-created.ksm"],
        "os error 2",
        true,
    )
}

#[test]
fn help() -> EmptyResult {
    test_with_args(
        vec!["--help"],
        "A small utility that disassembles and reads KSM and KO files for use with KerbalOS",
        false,
    )
}

#[test]
fn no_flags() -> EmptyResult {
    test_with_args(vec![KASH_PATH], "kDump version", false)
}

#[test]
fn full_contents() -> EmptyResult {
    test_with_args(vec![KASH_PATH, "--full-contents"], "kpp 1.1", false)
}
