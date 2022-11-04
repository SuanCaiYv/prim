#!/bin/zsh
echo "this a shell script for generate local-ip1 certificate and it's key. It should used only for local-ip1 development."

# generate cert-key:
eval "openssl req -config local-ip1.conf -new -sha256 -newkey rsa:2048 -nodes -keyout local-ip1.key.pem -x509 -days 365 -out local-ip1.cert.pem"
# convert cert
eval "openssl x509 -outform der -in local-ip1.cert.pem -out local-ip1.cert.der"
# convert key
eval "openssl rsa -inform pem -in local-ip1.key.pem -outform der -out local-ip1.key.der"
echo "now you have get your cert-key pair for local-ip1."