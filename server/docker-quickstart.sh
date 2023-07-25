#!/bin/zsh
# if you are using OrbStack as your docker engine front-end,
# please exit it and turn Docker-Desktop(for macOS and Windows) on.
# using OrbStack for launch this service need some additional work to be done.
# and it's hard to configure.
docker build -t prim:scheduler -f ./dockerfile-scheduler .
docker build -t prim:message -f ./dockerfile-message .
docker build -t prim:seqnum -f ./dockerfile-seqnum .
docker build -t prim:api -f ./dockerfile-api .

docker-compose up