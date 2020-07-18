#!/usr/bin/env bash
set -euo pipefail

DIR="/home/daniel/purpleifypdf"

function header() {
    printf "\n--- %s ---\n\n" "$1";
}

# Ensure you've run ./install-deps.sh

header "Building release"
cargo build --release

header "Uploading"

echo "Uploading binary"
rsync -e ssh --info=progress2 \
    $DIR/target/release/server \
    app-purpleifypdf@primary.server:/home/app-purpleifypdf/

echo "Uploading dummy PDF"
rsync -e ssh --info=progress2 \
    $DIR/test_assets/test_dummy_in.pdf \
    app-purpleifypdf@primary.server:/home/app-purpleifypdf/test_assets/

header "Restarting service"
ssh app-purpleifypdf@primary.server "systemctl --user restart purpleifypdf"

header "Status"

ssh app-purpleifypdf@primary.server "SYSTEMD_COLORS=1 systemctl --user status purpleifypdf"
printf "\n\n\nGET /purpleifypdf/version: %s\n\n" "$(curl --silent -L https://danielzfranklin.org/purpleifypdf/version)"
