#!/bin/zsh

docker build -t prim/scheduler-aarch64-linux -f ../docker/aarch64-linux/dockerfile-scheduler . &&
docker build -t prim/message-aarch64-linux -f ../docker/aarch64-linux/dockerfile-message . &&
docker build -t prim/seqnum-aarch64-linux -f ../docker/aarch64-linux/dockerfile-seqnum . &&
docker build -t prim/api-aarch64-linux -f ../docker/aarch64-linux/dockerfile-api . &&

docker tag prim/scheduler-aarch64-linux ghcr.io/suancaiyv/prim/scheduler-aarch64-linux:latest &&
docker tag prim/message-aarch64-linux ghcr.io/suancaiyv/prim/message-aarch64-linux:latest &&
docker tag prim/seqnum-aarch64-linux ghcr.io/suancaiyv/prim/seqnum-aarch64-linux:latest &&
docker tag prim/api-aarch64-linux ghcr.io/suancaiyv/prim/api-aarch64-linux:latest &&