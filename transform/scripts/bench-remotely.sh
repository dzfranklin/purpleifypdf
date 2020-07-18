#!/usr/bin/env bash

# Ensure you've run ./install-deps.sh and ./install-dev-deps.sh 

set -o pipefail

printf "Uploading source\n"
rsync -e ssh --info=progress2 --inplace \
    /home/daniel/purpleifypdf app-purpleifypdf@primary.server:/var/tmp/bench-purpleifypdf

printf "Compiling and running benchmarks\n"
ssh app-purpleifypdf@primary.server \
    "cd /var/tmp/bench-purpleifypdf/purpleifypdf && ~/.cargo/bin/cargo bench"
