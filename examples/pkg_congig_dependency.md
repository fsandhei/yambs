# pkg-config dependency

A project that is distributed as pkg-config module can be pulled in as a dependency to a `yambs` project.

For example, [`catch2`](https://github.com/catchorg/Catch2) distributes pkg-config files with their `vcpkg` port, which
subsequently can be used in yambs.

For an executable `x`, `catch2` can be pulled in as a dependency by specifying the pkg-config search directory

```toml
...

[executable.x.dependencies.catch2-with-main]
debug.pkg_config_search_dir = "/path/to/debug/pkg-config/file"
release.pkg_config_search_dir = "/path/to/release/pkg-config/file"

```
