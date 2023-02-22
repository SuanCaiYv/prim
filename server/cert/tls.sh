#! /bin/bash

if [ "$#" -ne 1 ]
then
  echo "Error: No domain name argument provided"
  echo "Usage: Provide a domain name as an argument"
  exit 1
fi

DOMAIN=$1

# Create root CA & Private key

openssl req -x509 \
            -sha256 -days 3560 \
            -nodes \
            -newkey rsa:2048 \
            -subj "/CN=${DOMAIN}/C=ZH/L=SH" \
            -keyout PrimRootCa.key -out PrimRootCa.crt

# Create csf conf

cat > csr.conf <<EOF
[ req ]
default_bits = 2048
prompt = no
default_md = sha256
req_extensions = req_ext
distinguished_name = dn

[ dn ]
C = ZH
ST = SH
L = SH
O = PRIM
OU = PRIM Dev
CN = ${DOMAIN}

[ req_ext ]
subjectAltName = @alt_names

[ alt_names ]
DNS.1 = ${DOMAIN}
DNS.2 = www.${DOMAIN}
IP.1 = 127.0.0.1

EOF

# Create a external config file for the certificate

cat > cert.conf <<EOF

authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage = digitalSignature, nonRepudiation, keyEncipherment, dataEncipherment
subjectAltName = @alt_names

[alt_names]
DNS.1 = ${DOMAIN}

EOF

# Generate Private key

openssl genrsa -out ${DOMAIN}-server.key 2048

# create CSR request using private key

openssl req -new -key ${DOMAIN}-server.key -out ${DOMAIN}-server.csr -config csr.conf

# Create SSl with self signed CA

openssl x509 -req \
    -in ${DOMAIN}-server.csr \
    -CA PrimRootCa.crt -CAkey PrimRootCa.key \
    -CAcreateserial -out ${DOMAIN}-server.crt \
    -days 3650 \
    -sha256 -extfile cert.conf

openssl genrsa -out ${DOMAIN}-server.key 2048

openssl req -new -key ${DOMAIN}-server.key -out ${DOMAIN}-server.csr -config csr.conf

# Generate Private key

openssl genrsa -out ${DOMAIN}-client.key 2048

# create CSR request using private key

openssl req -new -key ${DOMAIN}-client.key -out ${DOMAIN}-client.csr -config csr.conf

# Create SSl with self signed CA

openssl x509 -req \
    -in ${DOMAIN}-client.csr \
    -CA PrimRootCa.crt -CAkey PrimRootCa.key \
    -CAcreateserial -out ${DOMAIN}-client.crt \
    -days 3650 \
    -sha256 -extfile cert.conf

openssl genrsa -out ${DOMAIN}-client.key 2048

openssl req -new -key ${DOMAIN}-client.key -out ${DOMAIN}-client.csr -config csr.conf

eval "openssl x509 -outform der -in ${DOMAIN}-server.crt -out ${DOMAIN}-server.crt.der"

eval "openssl rsa -inform pem -in ${DOMAIN}-server.key -outform der -out ${DOMAIN}-server.key.der"

eval "openssl x509 -outform der -in ${DOMAIN}-client.crt -out ${DOMAIN}-client.crt.der"

eval "openssl rsa -inform pem -in ${DOMAIN}-client.key -outform der -out ${DOMAIN}-client.key.der"

# usage: ./tls.sh localhost