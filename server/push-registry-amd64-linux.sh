#!/bin/zsh

docker push ghcr.io/suancaiyv/prim/scheduler-amd64-linux:latest &&
docker push ghcr.io/suancaiyv/prim/message-amd64-linux:latest &&
docker push ghcr.io/suancaiyv/prim/seqnum-amd64-linux:latest &&
docker push ghcr.io/suancaiyv/prim/api-amd64-linux:latest