# KDump

[<img src="https://img.shields.io/badge/github-newcomb--luke%2FKDump-8da0cb?style=for-the-badge&logo=github&labelColor=555555" alt="github" height="24">](https://github.com/newcomb-luke/KDump)
[<img src="https://img.shields.io/crates/v/kdump?color=fc8d62&logo=rust&style=for-the-badge" alt="github" height="24">](https://crates.io/crates/kdump)
[<img alt="License" src="https://img.shields.io/github/license/newcomb-luke/KDump?style=for-the-badge" height="24">]()

[<img alt="GitHub Workflow Status" src="https://img.shields.io/github/actions/workflow/status/newcomb-luke/KDump/main.yml?style=for-the-badge" height="24">]()
[<img alt="Libraries.io dependency status for GitHub repo" src="https://img.shields.io/librariesio/github/newcomb-luke/KDump?style=for-the-badge" height="24">](https://deps.rs/repo/github/newcomb-luke/KDump)
[<img alt="Crates.io Downloads" src="https://img.shields.io/crates/d/kdump?style=for-the-badge" height="24">]()

KDump is a program that allows anyone with a command line to view the contents of KerboScript Machine Code (KSM) files, and KerbalObject (KO) files to view compiled code for Kerbal Operating System. KO and KSM files are fully supported.

KDump is the kOS equivalent to ELF's **objdump** or **readelf** programs.

![screenshot](https://github.com/newcomb-luke/KDump/blob/main/images/kdump.png)

## Features

* Color terminal output
* Human readable text
* Debug line number display alongside disassembly

## Installation

KDump can either be installed via [cargo](https://github.com/rust-lang/cargo) through [crates.io](https://crates.io), or as a standalone binary.

#### Windows

- Download the installer .msi file from Releases on the right
- Run the installer
- **kdump** should now be added to your PATH and available from any CMD or Powershell window

#### Arch Linux

* Download the PKGBUILD from Releases on the right

* Copy it to a temporary folder

* Run `makepkg -si` to install the **kdump** and all of its dependencies.

* **kdump** should now be added to your PATH and available from any terminal

#### Standalone Executables

- Download and extract the .zip file from Releases on the right
- Place the executable in the desired location
- Run the executable through the terminal, Powershell on Windows or the default terminal on Mac OS or Linux.

#### Cargo

To install using **cargo**:

```
cargo install kdump
```

`kdump` should then be added to your shell's PATH, and can be run from any terminal

## Usage

KDump can be invoked after installation as `kdump`

Help can be accessed from the program itself by running:

```
kdump --help
```

KDump takes one .ko or .ksm file as input:

```
kdump program.ksm
```

However the default mode is to print nothing. Print options need to be specified.

Most of the time the most useful, even if it takes up the most space is to view the file's full contents. This can be specified by using the **-x** or **--full-contents** flags:

```
kdump program.ksm --full-contents
```

If only disassembly of function sections is the main concern, then use only the **-D** or **--disassemble** flags:

```
kdump lib.ko -D
```

When debugging compiled KerboScript files, debug information is stored in KSM files, and KDump is able to read and display this information. Line numbers are displayed to the left of disassembly and can be enabled using the **-l** or **--line-numbers** flags:

```
kdump launch.ksm -Dl
```

If a file was generated using the [Kerbal Linker](https://github.com/newcomb-luke/kOS-KLinker) or similar programs, then there may be information about what tools were used to generate KerbalObject and KSM files, and this can be viewed by passing the **-i** or **--info** flags:

```
kdump script.ksm -i
```
