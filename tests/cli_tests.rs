use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

type EmptyResult = Result<(), Box<dyn std::error::Error>>;

static KASH_PATH: &str = "tests/inputs/kash.ksm";
static KO_PATH: &str = "tests/inputs/test.ko";
static GARBAGE_PATH: &str = "tests/inputs/garbage.ksm";

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
    expected_output: &str,
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

    println!("Expected: {}", expected_output);
    println!("Found: {}", &output_1);

    assert!(predicate::str::contains(expected_output).eval(&output_1));

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
fn full_contents_ksm() -> EmptyResult {
    test_long_and_short(vec![KASH_PATH], "--full-contents", "-s", "kpp 1.1")
}

#[test]
fn disassemble_ksm() -> EmptyResult {
    test_long_and_short(
        vec![KASH_PATH],
        "--disassemble",
        "-D",
        "@002206 4c 09fe 0136  call  \"\",\"<indirect>\"",
    )
}

#[test]
fn disassemble_symbol_ksm() -> EmptyResult {
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
        vec![KASH_PATH, "-D"],
        "--line-numbers",
        "-l",
        "   389 ═╣  @001950 4e 0cc5       push  \"kash: Duplicate job setup error\"",
    )
}

#[test]
fn info_ksm() -> EmptyResult {
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
        "\n  @002205   pdrl  \"@1184\",true",
        false,
    )
}

#[test]
fn show_no_labels_ksm() -> EmptyResult {
    test_with_args(
        vec![KASH_PATH, "-D", "--show-no-labels"],
        "\n  4c 09fe 0136  call  \"\",\"<indirect>\"",
        false,
    )
}

#[test]
fn disassemble_ko() -> EmptyResult {
    test_long_and_short(
        vec![KO_PATH],
        "--disassemble",
        "-D",
        "  00000002 4e ffffffff           push <five>",
    )
}

#[test]
fn disassemble_symbol_ko() -> EmptyResult {
    let expected = format!("kDump version {}\nreally_long_name_with_underscores:\n  00000001 4d 00000004           ret  0", kdump::VERSION);
    test_long_and_short(
        vec![KO_PATH],
        "--disassemble-symbol=really_long_name_with_underscores",
        "-d=really_long_name_with_underscores",
        &expected,
    )
}

#[test]
fn file_headers() -> EmptyResult {
    test_long_and_short(
        vec![KO_PATH],
        "--file-headers",
        "-f",
        "File header:\n\tVersion: 4\n\tShstrtab Index: 1\n\tNumber of section headers: 9",
    )
}

#[test]
fn section_headers() -> EmptyResult {
    test_with_args(
        vec![KO_PATH, "--section-headers"],
        r#"Sections:
Index  Name            Kind        Size        
0                       NULL        0           

1      .shstrtab        STRTAB      92          

2      .data            DATA        20          

3      .symtab          SYMTAB      56          

4      .comment         STRTAB      24          

5      .symstrtab       STRTAB      57          

6      .reld            RELD        11          

7      _start           FUNC        27          

8      really_long_name FUNC        5"#,
        false,
    )
}

#[test]
fn data_sections() -> EmptyResult {
    test_with_args(
        vec![KO_PATH, "--data"],
        r#"Symbol Data Sections:
Section .data
Index       Type        Value
  0         NULL
  1         ARGMARKER   
  2         STRING      "$two"
  3         STRING      "print()"
  4         INT16       0"#,
        false,
    )
}

#[test]
fn full_contents_ko() -> EmptyResult {
    test_long_and_short(
        vec![KO_PATH],
        "--full-contents",
        "-s",
        r#"Relocation data sections:
Reld section .reld:
Section     Instruction   Operand     Symbol index
7           00000001      1           00000002

Function sections: 
_start:
  00000001 4e 00000001           push @"#,
    )
}

#[test]
fn stabs() -> EmptyResult {
    test_long_and_short(
        vec![KO_PATH],
        "--stabs",
        "-S",
        ".symstrtab\n  [    1]  test.kasm",
    )
}

#[test]
fn syms() -> EmptyResult {
    test_long_and_short(
        vec![KO_PATH],
        "--syms",
        "-t",
        "test.kasm        ffffffff  0000    GLOBAL    FILE      0",
    )
}

#[test]
fn reloc() -> EmptyResult {
    test_long_and_short(
        vec![KO_PATH],
        "--reloc",
        "-r",
        "7           00000001      1           00000002",
    )
}

#[test]
fn info_ko() -> EmptyResult {
    test_long_and_short(vec![KO_PATH], "--info", "-i", "Compiled by KASM")
}

#[test]
fn show_no_label_ko() -> EmptyResult {
    test_with_args(
        vec![KO_PATH, "-D", "--show-no-labels"],
        "\n  4e 00000001           push @",
        false,
    )
}

#[test]
fn show_no_raw_instr_ko() -> EmptyResult {
    test_with_args(
        vec![KO_PATH, "-D", "--show-no-raw-instr"],
        "\n  00000001  push @",
        false,
    )
}

#[test]
fn garbage_input() -> EmptyResult {
    test_with_args(
        vec![GARBAGE_PATH, "--full-contents"],
        "File type not recognized",
        true,
    )
}
