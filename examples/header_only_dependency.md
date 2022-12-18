# Header only dependency

A header only dependency can be added to a YAMBS project. It is added through the dependency table as
like the other dependency objects in YAMBS.

To declare a header only dependency, you have to write the following:

```toml
...
[executable.x.dependencies.<dependency>]
include_directory = "<include/directory>"
```

Note that a header only dependency does not create a build target.
