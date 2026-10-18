#!/usr/bin/env bash
# One-time: creates a local, self-signed code-signing identity
# "TheOrbit Local Signing" in your login keychain. Signing every build with
# the same identity lets macOS keep TheOrbit's Accessibility permission
# across rebuilds (ad-hoc signatures change on every build).
set -euo pipefail
NAME="TheOrbit Local Signing"

if security find-identity -p codesigning | grep -q "$NAME"; then
  echo "Identity \"$NAME\" already exists."
  exit 0
fi

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
cat > "$WORK/cert.cnf" <<CNF
[req]
distinguished_name = dn
prompt = no
x509_extensions = ext
[dn]
CN = $NAME
[ext]
basicConstraints = critical,CA:false
keyUsage = critical,digitalSignature
extendedKeyUsage = critical,codeSigning
CNF

openssl req -x509 -newkey rsa:2048 -nodes -days 3650 \
  -keyout "$WORK/key.pem" -out "$WORK/cert.pem" -config "$WORK/cert.cnf" 2>/dev/null
# macOS only reads the older PKCS#12 algorithms. A throwaway password: the .p12 only exists inside $WORK for the import.
openssl pkcs12 -export -inkey "$WORK/key.pem" -in "$WORK/cert.pem" \
  -out "$WORK/id.p12" -passout pass:orbit -name "$NAME" \
  -keypbe PBE-SHA1-3DES -certpbe PBE-SHA1-3DES -macalg sha1 2>/dev/null

security import "$WORK/id.p12" -k "$HOME/Library/Keychains/login.keychain-db" \
  -P orbit -T /usr/bin/codesign >/dev/null
echo "Created \"$NAME\" in your login keychain."
