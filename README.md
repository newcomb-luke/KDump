# KDump

[![Libraries.io dependency status for GitHub repo](https://img.shields.io/librariesio/github/newcomb-luke/KDump)](https://deps.rs/repo/github/newcomb-luke/KDump)
![GitHub](https://img.shields.io/github/license/newcomb-luke/KDump)

![Crates.io](https://img.shields.io/crates/v/kdump?color=%235555cc)
![Crates.io](https://img.shields.io/crates/d/kdump)

KDump is a program that allows anyone with a command line to view the contents of KerboScript Machine Code (KSM) files, and KerbalObject (KO) files to view compiled code for Kerbal Operating System. KO and KSM files are fully supported.

KDump is the kOS equivalent to ELF's **objdump** or **readelf** programs.

## Features

* Color terminal output
* Human readable text
* Debug line number display alongside disassembly

## Installation

KDump can either be installed via [cargo](https://github.com/rust-lang/cargo) through [crates.io](https://crates.io), or as a standalone binary.

To install using **cargo**:
```
cargo install kdump
```

`kdump` should then be added to your shell's PATH, and can be run from any terminal

To install using the standalone binaries:
* Download and extract the .zip file from Releases on the right
* Place the executable in the desired location
* Run the executable through the terminal, Powershell on Windows or the default terminal on Mac OS or Linux.

To install using the Windows installer:
* Download the installer .msi file from Releases on the right
* Run the installer
* `kdump` should now be added to your PATH and available from any CMD or Powershell window

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
