# YAMBS C++ Meta Build System

YAMBS (Yet Another Meta Build System) is a command-line based meta build system for C++, written in Rust.

## Motivation
It is intended to be a quick and easy build system for developers to easily maintain and manage their project.

The idea is that development of a C++ project with YAMBS shall have focus on the C++ language, without the need of knowing a separate
scripting language for maintaining your project.

## How it works
YAMBS is a simple system. From a manifest file that contains all files that is required to build your project, YAMBS generates
build files for your project and then builds your project from that. That's it.

By writing the minimal amount of boilerplate code, a user gets a well equipped build environment with easily modifiable runtime configurations.

Diagram here of how a YAMBS hierarchical project tree should look like and so on...

## Getting started
YAMBS requires a manifest file located in the root of the project. The manifest is a TOML file named `yambs.toml`.

The simplest example of using YAMBS with a single C++ file, `main.cpp` that yields the executable `x` requires the following content

```toml
[executable.x]
sources = ["main.cpp"]
```

Before building, YAMBS requires that the following environment variables are set:

* CXX (supports clang++ and g++)
* YAMBS_BUILD_SYSTEM_EXECUTABLE (only allowed value is the path to `make` for now)

Finally, to build the project you invoke the meta build system like this:

```bash
yambs build -b build
```

YAMBS will generate the necessary build files for the project in debug configuration and then build the project.
The program produces the following directory tree:

```
build
├── cache
│   ├── compiler
│   ├── manifest
│   └── targets
├── debug
│   ├── main.d
│   ├── main.o
│   ├── Makefile
│   ├── progress.json
│   └── x
├── make_include
│   ├── debug.mk
│   ├── default_make.mk
│   ├── defines.mk
│   ├── release.mk
│   └── strict.mk
├── sample
│   ├── a.out
│   └── main.cpp
└── yambs_log.txt
```

## Manifest
The manifest is a TOML file that must contain targets. The targets can be executables or libraries.
A target is defined as a map entry in TOML land.

The manifest abides [TOML v0.5.0](https://toml.io/en/v0.5.0).

### Syntax
An executable is formed with the syntax:
```
[executable.<name>]
```
Similarily, a library is formed with:
```
[library.<name>]
```

A target accepts the following fields:
* `sources`: An array of strings of file paths.
* `cxxflags_append`: An array of strings that passes additional CXX flags for that target.
* `cppflags_append`: An array of strings that passes additional CPP flags for that target.
* `dependencies`: A table specifying the projects this target depends on.
   * Dependencies can be of two types
      * From source: Specify a dependency as a YAMBS project. Currently this is supported as a project on your filesystem.
      * From binary: Specify a binary to be used as a dependency.

A library has an additional field:
* `type`: String specifing if this library is a static or shared library.
   * Allowed values: "shared", "static".
   * Default: "static"
