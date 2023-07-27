#!/bin/zsh

docker build -t prim/scheduler-amd64-linux -f ../docker/amd64-linux/dockerfile-scheduler . &&
docker build -t prim/message-amd64-linux -f ../docker/amd64-linux/dockerfile-message . &&
docker build -t prim/seqnum-amd64-linux -f ../docker/amd64-linux/dockerfile-seqnum . &&
docker build -t prim/api-amd64-linux -f ../docker/amd64-linux/dockerfile-api . &&

#docker tag prim/scheduler-amd64-linux ghcr.io/suancaiyv/prim/scheduler-amd64-linux:latest &&
#docker tag prim/message-amd64-linux ghcr.io/suancaiyv/prim/message-amd64-linux:latest &&
#docker tag prim/seqnum-amd64-linux ghcr.io/suancaiyv/prim/seqnum-amd64-linux:latest &&
#docker tag prim/api-amd64-linux ghcr.io/suancaiyv/prim/api-amd64-linux:latest &&