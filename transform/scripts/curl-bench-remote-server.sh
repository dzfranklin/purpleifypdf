#!/usr/bin/env bash

hyperfine "curl --data-binary '@test_assets/multipage_test.pdf' https://danielzfranklin.org/purpleifypdf/transform?meta=%7B%22clientUid%22%3A%20%229999%22%2C%20%22quality%22%3A%20%22High%22%2C%20%22backgroundColor%22%3A%20%7B%22r%22%3A%20255%2C%20%22g%22%3A%20100%2C%20%22b%22%3A%2050%7D%2C%20%22source%22%3A%20%22file%3A%2F%2Ffake%22%7D --output /dev/null"
