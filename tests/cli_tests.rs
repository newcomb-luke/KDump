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

fn test_long_and_short(
    other_args: Vec<&'static str>,
    long: &'static str,
    short: &'static str,
    expected_output: &'static str,
) -> EmptyResult {
    let mut cmd = Command::cargo_bin("kdump")?;

    for arg in &other_args {
        cmd.arg(arg);
    }

    cmd.arg(long);

    let output_1 = String::from_utf8(cmd.assert().success().get_output().stdout.clone()).unwrap();

    let mut cmd = Command::cargo_bin("kdump")?;

    for arg in other_args {
        cmd.arg(arg);
    }

    cmd.arg(short);

    let output_2 = String::from_utf8(cmd.assert().success().get_output().stdout.clone()).unwrap();

    assert_eq!(output_1, output_2);

    predicate::str::contains(expected_output).eval(&output_1);

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

#[test]
fn disassemble() -> EmptyResult {
    test_long_and_short(
        vec![KASH_PATH],
        "--disassemble",
        "-D",
        "@002206 4c 09fe 0136  call  \"\",\"<indirect>\"",
    )
}

#[test]
fn disassemble_symbol() -> EmptyResult {
    test_long_and_short(
        vec![KASH_PATH],
        "--disassemble-symbol=main",
        "-d=main",
        "@002206 4c 09fe 0136  call  \"\",\"<indirect>\"",
    )
}

#[test]
fn arg_section() -> EmptyResult {
    test_long_and_short(vec![KASH_PATH], "--argument-section", "-a", "kpp 1.1")
}

#[test]
fn line_numbers() -> EmptyResult {
    test_long_and_short(
        vec![KASH_PATH],
        "--line-numbers",
        "-l",
        "   389 ═╣  @001950 4e 0cc5       push  \"kash: Duplicate job setup error\"",
    )
}

#[test]
fn info() -> EmptyResult {
    test_long_and_short(
        vec![KASH_PATH],
        "--info",
        "-i",
        "Compiled using official kOS compiler.",
    )
}

#[test]
fn show_no_raw_instr_ksm() -> EmptyResult {
    test_with_args(
        vec![KASH_PATH, "-D", "--show-no-raw-instr"],
        "  @002205   pdrl  \"@1184\",true",
        false,
    )
}

#[test]
fn show_no_labels_ksm() -> EmptyResult {
    test_with_args(
        vec![KASH_PATH, "-D", "--show-no-labels"],
        "  4c 09fe 0136  call  \"\",\"<indirect>\"",
        false,
    )
}
