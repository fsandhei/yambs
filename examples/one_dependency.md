# One dependency

With a project tree like this,

```bash
├── include
│   ├── FileSubscriber.hpp
│   ├── ISubject.hpp
│   ├── ISubscriber.hpp
│   └── Subject.hpp
├── src
│   ├── FileSubscriber.cpp
│   ├── main.cpp
│   └── Subject.cpp
```

and a dependency project (also written in YAMBS), `visitorlibrary`, located in a directory adjacent to this project, then the manifest file is written like this:

```toml
[executable.x]
sources = ["src/main.cpp", "src/Subject.cpp", "src/FileSubscriber.cpp"]

[executable.x.dependencies]
visitorlibrary = { path = "../visitorlibrary" }
```
