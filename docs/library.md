# Library entry

The library table entry in the manifest creates a build file target for a C++ library.

Multiple libraries can be added in the same manifest.

## Example
```toml
[library.<name>]
sources = [...]
cxxflags_append = [...]
cppflags_append = [...]
type = "static|shared"

[library.<name>.defines]
macro = "..."
value = "..."

[library.<name>.dependencies]
...
```
