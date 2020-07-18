#!/usr/bin/env bash

apt install -y \
    clang \
    llvm

curl https://sh.rustup.rs -sSf | sh -s -- -y
