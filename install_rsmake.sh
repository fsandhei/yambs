#! /bin/bash

set -e # Bail on error.

install_mymake()
{
   echo "Installing release build of RsMake into $BIN_DIR"
   cp -f -v $RSMAKE_RELEASE $BIN_DIR
}
