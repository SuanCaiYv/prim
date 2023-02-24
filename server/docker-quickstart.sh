#!/bin/zsh
docker build -t prim:scheduler -f ./dockerfile-scheduler .
docker build -t prim:recorder -f ./dockerfile-recorder .
docker build -t prim:message -f ./dockerfile-message .
docker build -t prim:api -f ./dockerfile-api .

docker-compose up