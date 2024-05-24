#! /usr/bin/env bash

set -euo pipefail

HERE=$(dirname $(readlink -f "$BASH_SOURCE"))

rustc "$HERE/executable-packer.rs" -o "$HERE/executable-packer"
