#!/bin/zsh

# generate cert-key:
eval "openssl req -config localhost.conf -new -sha256 -newkey rsa:2048 -nodes -keyout localhost.key.pem -x509 -days 365 -out localhost.cert.pem"
# convert cert
eval "openssl x509 -outform der -in localhost.cert.pem -out localhost.cert.der"
# convert key
eval "openssl rsa -inform pem -in localhost.key.pem -outform der -out localhost.key.der"
echo "now you have get your cert-key pair for localhost."