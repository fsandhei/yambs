#!/bin/bash

set -e # Bail on error.

CARGO="/home/fredrik/.cargo/bin/cargo"
ROOT_DIR="/home/fredrik/bin/mymake"
MYMAKE="$ROOT_DIR/target/debug/mymake"
CWD=`pwd`

build_mymake()
{
   echo "Building latest version of MyMake."
   cd $ROOT_DIR
   $CARGO build
}


test_mymake_minimal_build()
{
   TEST_DIR="$ROOT_DIR/test_project"
   mkdir $TEST_DIR && cd $TEST_DIR
   mkdir "$TEST_DIR/src"

   cat << EOF > $TEST_DIR/src/test.cpp
#include <iostream>

int main()
{
   std::cout << "Minimum build test successful!\n";
}

EOF

cat << EOF > $TEST_DIR/run.mmk
MMK_EXECUTABLE:
   x

MMK_SOURCES:
   src/test.cpp
EOF

   cd $ROOT_DIR && build_mymake
   mkdir "$ROOT_DIR/build" && cd $ROOT_DIR/build
   $MYMAKE -g "$TEST_DIR/run.mmk" && "$ROOT_DIR/build/release/x"
   build_result=$?
   if [ "$build_result" -ne 0 ]; then
      return "$build_result"
   fi
   cd "$ROOT_DIR"
   rm -rf "$ROOT_DIR/build" "$TEST_DIR"
}


test_mymake_with_one_dependency_build()
{
   TEST_DIR="$ROOT_DIR/test_project"
   TEST_DIR_DEP="$ROOT_DIR/test_dependency_project"
   mkdir $TEST_DIR && cd $TEST_DIR
   mkdir "$TEST_DIR/src"

   cat << EOF > $TEST_DIR/src/test.cpp
#include <iostream>
#include <example.h>

int main()
{
   std::cout << "Minimum build with one dependency test successful!\n";
}

EOF

cat << EOF > $TEST_DIR/run.mmk
MMK_DEPEND:
  $TEST_DIR_DEP/src 

MMK_EXECUTABLE:
   x

MMK_SOURCES:
   src/test.cpp
EOF

   mkdir $TEST_DIR_DEP && cd $TEST_DIR_DEP
   mkdir "$TEST_DIR_DEP/src"
   mkdir "$TEST_DIR_DEP/include"

   cat << EOF > $TEST_DIR_DEP/src/example.cpp
#include "../include/example.h"

bool A::hello() const
{
   return true;
}
EOF

   cat << EOF > $TEST_DIR_DEP/include/example.h

#ifndef EXAMPLE_H
#define EXAMPLE_H

class A
{
   public:
      bool hello() const;
};

#endif
EOF

   cat << EOF > $TEST_DIR_DEP/src/lib.mmk

MMK_LIBRARY_LABEL:
   example_library

MMK_SOURCES:
   example.cpp

EOF
   mkdir "$ROOT_DIR/build" && cd "$ROOT_DIR/build"
   $MYMAKE -g "$TEST_DIR/run.mmk" && "$ROOT_DIR/build/release/x"
   build_result=$?
   if [ "$build_result" -ne 0 ]; then
      return "$build_result"
   fi
   cd "$ROOT_DIR"
   rm -rf "$ROOT_DIR/build" "$TEST_DIR" "$TEST_DIR_DEP"
}


execute_command() 
{
   $@
   if [ $? -gt 0 ]; then 
      echo "$@ FAILED. Aborting..."
      exit 1
   fi
}


cargo_test()
{
   path=$1
   cd $path
   echo "cargo test -p $path"
   execute_command "$CARGO test"
   cd $ROOT_DIR
}

[ $CWD != $ROOT_DIR ] && cd $ROOT_DIR

cargo_test "${ROOT_DIR}/mmk_parser"
cargo_test "${ROOT_DIR}/builder"
cargo_test "${ROOT_DIR}/dependency"
cargo_test "${ROOT_DIR}/generator"

test_mymake_minimal_build
test_mymake_with_one_dependency_build

if [ "$?" -eq 0 ]; then
   echo "SUCCESS"
else
   echo "FAILURE"
fi
exit "$?"
