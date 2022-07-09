#!/bin/bash

set -e # Bail on error.

trap "remove_build_and_test_files $?" EXIT
trap "remove_build_and_test_files $?" INT

ACCEPTANCE_TESTS_ONLY="false"
BIN_DIR="/usr/bin/"
CARGO="/home/fredrik/.cargo/bin/cargo"
ROOT_DIR="/home/fredrik/dev/private/rsmake"
RSMAKE="$ROOT_DIR/target/release/rsmake"
RSMAKE_RELEASE="$ROOT_DIR/target/release/rsmake"
CWD=`pwd`
   
TEST_DIR="$ROOT_DIR/test_project"
TEST_DIR_DEP="$ROOT_DIR/test_dependency_project"

usage() {
   echo "$(basename $0)"
   echo "Regression test sript to run all tests for RsMake."
   echo "Usage:"
   echo "   $(basename $0) [--acceptance-tests | -h | --help]"
   echo "Flags:"
   echo "  --acceptance-tests   Only run acceptance tests, skipping unit tests."
   echo "  -h, --help           Display help message and exit."
}

set_CXX() {
   export CXX="$1"
}

remove_build_and_test_files() {
   if [[ "$1" != "0" ]]; then
      echo "An error occured. Cleaning up and exiting..."
   fi
   rm -rf "$ROOT_DIR/build"
   rm -rf "$TEST_DIR"
   rm -rf "$TEST_DIR_DEP"
}

build_mymake()
{
   echo "Building latest version of RsMake."
   cd $ROOT_DIR
   $CARGO build --release
}


create_dummy_project() {
   if [ -d "$TEST_DIR" ]; then
      rm -rf "$TEST_DIR"
   fi
   mkdir $TEST_DIR && cd $TEST_DIR
   mkdir "$TEST_DIR/src"
}


create_dummy_library() {
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
}


test_mymake_minimal_build()
{
   create_dummy_project 
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

   if [ -d "$ROOT_DIR/build" ]; then
      rm -rf "$ROOT_DIR/build"
   fi
   mkdir "$ROOT_DIR/build" && cd "$ROOT_DIR/build"
   "$RSMAKE" -g "$TEST_DIR/run.mmk" && "$ROOT_DIR/build/release/x"
   build_result=$?
   if [ "$build_result" -ne 0 ]; then
      return "$build_result"
   fi
   cd "$ROOT_DIR"
   rm -rf "$ROOT_DIR/build" "$TEST_DIR"
}


test_mymake_minimal_build_with_explicit_cpp_version_and_implicit_release()
{
   create_dummy_project 
   cat << EOF > $TEST_DIR/src/test.cpp

int main()
{
}
EOF
   cat << EOF > $TEST_DIR/run.mmk
MMK_EXECUTABLE:
   x

MMK_SOURCES:
   src/test.cpp
EOF

   if [ -d "$ROOT_DIR/build" ]; then
      rm -rf "$ROOT_DIR/build"
   fi
   mkdir "$ROOT_DIR/build" && cd "$ROOT_DIR/build"
   "$RSMAKE" -g "$TEST_DIR/run.mmk" -c "c++17" && "$ROOT_DIR/build/release/x"
   build_result=$?
   if [ "$build_result" -ne 0 ]; then
      return "$build_result"
   fi
   cd "$ROOT_DIR"
   rm -rf "$ROOT_DIR/build" "$TEST_DIR"
}



test_mymake_with_one_dependency_build()
{
   create_dummy_project 
   cat << EOF > $TEST_DIR/src/test.cpp
#include <iostream>
#include <example.h>

int main()
{
   A a;
   a.hello();
   std::cout << "Minimum build with one dependency test successful!\n";
}

EOF
   create_dummy_library

   cat << EOF > $TEST_DIR/run.mmk
MMK_REQUIRE:
  $TEST_DIR_DEP/src 

MMK_EXECUTABLE:
   x

MMK_SOURCES:
   src/test.cpp
EOF


   cat << EOF > $TEST_DIR_DEP/src/lib.mmk

MMK_LIBRARY_LABEL:
   example_library

MMK_SOURCES:
   example.cpp

EOF
   if [ -d "$ROOT_DIR/build" ]; then
      rm -rf "$ROOT_DIR/build"
   fi
   mkdir "$ROOT_DIR/build" && cd "$ROOT_DIR/build"
   "$RSMAKE" -g "$TEST_DIR/run.mmk" && "$ROOT_DIR/build/release/x"
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

run_rsmake_test()
{
   echo "$1"
   "$1" 1> /dev/null
}

while :; do
   case $1 in
      --acceptance-tests)
         ACCEPTANCE_TESTS_ONLY="true"
         ;;
      -h | --help)
         usage
         exit;;
      *)
         break
   esac
   shift
done

if [[ "$ACCEPTANCE_TESTS_ONLY" == "false" ]]; then
   [ $CWD != $ROOT_DIR ] && cd $ROOT_DIR
   cargo -q test
fi
   
cd $ROOT_DIR && build_mymake
gcc_exe=`which g++`
set_CXX "$gcc_exe"
echo "--------------------------- RUNNING ACCEPTANCE TESTS ---------------------------"
echo "CXX = $gcc_exe"
run_rsmake_test test_mymake_minimal_build
run_rsmake_test test_mymake_minimal_build_with_explicit_cpp_version_and_implicit_release
run_rsmake_test test_mymake_with_one_dependency_build

clang_exe=`which clang++`
set_CXX "$clang_exe"
echo "CXX = $clang_exe"
run_rsmake_test test_mymake_minimal_build
run_rsmake_test test_mymake_minimal_build_with_explicit_cpp_version_and_implicit_release
run_rsmake_test test_mymake_with_one_dependency_build
echo "--------------------------- END OF ACCEPTANCE TESTS ---------------------------"

if [ "$?" -eq 0 ]; then
   echo "SUCCESS"
else
   echo "FAILURE"
fi
exit "$?"
