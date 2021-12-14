#! /bin/bash

set -e # Bail on error.

BIN_DIR="/usr/bin/"
RSMAKE_RELEASE="$ROOT_DIR/target/release/rsmake"

install_mymake()
{
   echo "Installing release build of RsMake into $BIN_DIR"
   cp -f -v $RSMAKE_RELEASE $BIN_DIR
}
