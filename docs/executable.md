# Executable entry

The executable table entry in the manifest creates a build file target for a C++ executable.

Multiple executables can be added in the same manifest.

## Example
```toml
[executable.<name>]
sources = [...]
cxxflags_append = [...]
cppflags_append = [...]

[executable.<name>.defines]
macro = "..."
value = "..."

[executable.<name>.dependencies]
...
```
