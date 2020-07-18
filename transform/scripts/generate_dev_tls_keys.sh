#!/usr/bin/env bash

# See <https://letsencrypt.org/docs/certificates-for-localhost/>

openssl req -x509 -out test_assets/localhost.crt -keyout test_assets/localhost.key \
  -newkey rsa:2048 -nodes -sha256 \
  -subj '/CN=localhost' -extensions EXT -config <( \
   printf "[dn]\nCN=localhost\n[req]\ndistinguished_name = dn\n[EXT]\nsubjectAltName=DNS:localhost\nkeyUsage=digitalSignature\nextendedKeyUsage=serverAuth")
