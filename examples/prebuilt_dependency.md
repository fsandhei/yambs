# Prebuilt dependency

* TODO: Currently this is the only way of adding a vcpkg package into a YAMBS project. Should there be a feature for this?

If a project has a dependency that comes from `vcpkg`, then that dependency can be used in a YAMBS project.
The "downside" of using a binary dependency in YAMBS like so is that it involves some boilerplate code.

A prebuilt dependency requires some different information compared to a source dependency. It needs to know where to find a debug
and release configuration, respectively, as well as the include directory.

The following project examples uses GTest and GMock for testing and pulls in those libraries from vcpkg.

With the project tree as below,
```
├── include
│   ├── PlanGenerator.h
│   ├── Qgc_Item.h
│   ├── Qgc_Mission.h
│   └── Qgc_Util_OutStream.h
├── source
│   ├── Generator.cpp
│   ├── Generator.h
│   ├── PlanGenerator.cpp
│   ├── Qgc_Mission.cpp
│   ├── Qgc_Util_IndentingStreamBuf.cpp
│   ├── Qgc_Util_IndentingStreamBuf.h
│   └── Qgc_Util_OutStream.cpp
├── test
│   ├── CMakeLists.txt
│   ├── PlanGeneratorTest.cpp
│   ├── Qgc_OutStreamTest.cpp
└── yambs.toml
```
, then GTest and GMock can be pulled in like so:

```toml
[library.PlanGenerator]
sources = ["source/PlanGenerator.cpp",
           "source/Qgc_Util_OutStream.cpp",
           "source/Qgc_Mission.cpp",
           "source/Qgc_Util_IndentingStreamBuf.cpp",
           "source/Generator.cpp" ]

[executable.PlanGeneratorTests]
sources = ["test/PlanGeneratorTest.cpp", "test/Qgc_OutStreamTest.cpp"]

# PlanGeneratorTest needs to know where the library it is going to test comes from.
# See it as a "separate" library that you're using in your project.
# Note that this dependency can also be written as the following:

# [executable.PlanGeneratorTests.dependencies]
# PlanGenerator = { path = "." }
#
# The second format is just for consistency here.
[executable.PlanGeneratorTests.dependencies.PlanGenerator]
path = "."

[executable.PlanGeneratorTests.dependencies.gtest_lib]
debug.binary_path = "<vcpkg/root/path>/installed/x64-linux/debug/lib/libgtestd.a"
release.binary_path = "<vcpkg/root/path>/installed/x64-linux/lib/libgtest.a"
include_directory = "<vcpkg/root/path>/installed/x64-linux/include/"

[executable.PlanGeneratorTests.dependencies.gmock_lib]
debug.binary_path = "<vcpkg/root/path>/installed/x64-linux/debug/lib/libgmockd.a"
release.binary_path = "<vcpkg/root/path>/installed/x64-linux/lib/libgmock.a"
include_directory = "<vcpkg/root/path>/installed/x64-linux/include/"

[executable.PlanGeneratorTests.dependencies.gtest]
include_directory = "<vcpkg/root/path>/installed/x64-linux/include/"
debug.binary_path = "<vcpkg/root/path>/installed/x64-linux/debug/lib/manual-link/libgtest_maind.a"
release.binary_path = "<vcpkg/root/path>/installed/x64-linux/lib/manual-link/libgtest_main.a"

[executable.PlanGeneratorTests.dependencies.gmock]
include_directory = "<vcpkg/root/path>/installed/x64-linux/include/"
debug.binary_path = "<vcpkg/root/path>/installed/x64-linux/debug/lib/manual-link/libgmock_maind.a"
release.binary_path = "<vcpkg/root/path>/installed/x64-linux/lib/manual-link/libgmock_main.a"
```
