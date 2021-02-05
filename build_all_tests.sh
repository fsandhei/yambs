#!/bin/sh

CARGO=/home/fredrik/.cargo/bin/cargo
ROOT_DIR="/home/fredrik/bin/mymake/"
CWD=`pwd`

print_and_execute_command() 
{
   echo "$@"
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
   print_and_execute_command "$CARGO test"
   cd $ROOT_DIR
}

[ $CWD != $ROOT_DIR ] && cd $ROOT_DIR

cargo_test "${ROOT_DIR}mmk_parser"
cargo_test "${ROOT_DIR}builder"
cargo_test "${ROOT_DIR}dependency"
cargo_test "${ROOT_DIR}generator"

if [ "$?" -eq 0 ]; then
   echo "SUCCESS"
   exit "$?"
fi
