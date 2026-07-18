#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
    echo "usage: $0 SOURCE_DIR DEST_DIR" >&2
    exit 2
fi

source_dir=$1
dest_dir=$2

if [ -e "$dest_dir" ]; then
    echo "destination already exists: $dest_dir" >&2
    exit 1
fi

cp -a "$source_dir" "$dest_dir"
sed -i 's/^OPTFLAGS.*/OPTFLAGS   = -O0 -fno-common/' "$dest_dir/Makefile.vars"
sed -i 's/^DEBUGGER.*/DEBUGGER   = -g/' "$dest_dir/Makefile.vars"
make -C "$dest_dir" clean
make -C "$dest_dir" -j2
