#!/bin/bash
echo "Starting redis cluster...make sure you have installed redis-server on you host."

lsof -i tcp:16379-16381 | awk 'NR!=1 {print $2}' | xargs kill -9

rm -rf ./16379; rm -rf ./16380; rm -rf ./16381

mkdir -p ./16379; mkdir -p ./16380; mkdir -p ./16381

touch ./16379/redis.conf; touch ./16380/redis.conf; touch ./16381/redis.conf

echo "port 16379
bind 0.0.0.0
cluster-enabled yes
cluster-config-file ./16379/nodes.conf
cluster-node-timeout 5000
requirepass Redis.123456
appendonly yes
appendfilename '16379.aof'" >> ./16379/redis.conf

echo "port 16380
bind 0.0.0.0
cluster-enabled yes
cluster-config-file ./16380/nodes.conf
cluster-node-timeout 5000
requirepass Redis.123456
appendonly yes
appendfilename '16380.aof'" >> ./16380/redis.conf

echo "port 16381
bind 0.0.0.0
cluster-enabled yes
cluster-config-file ./16381/nodes.conf
cluster-node-timeout 5000
requirepass Redis.123456
appendonly yes
appendfilename '16381.aof'" >> ./16381/redis.conf

nohup redis-server ./16379/redis.conf > 16379.log &
nohup redis-server ./16380/redis.conf > 16380.log &
nohup redis-server ./16381/redis.conf > 16381.log &

sleep 3

redis-cli -p 16379 -a "Redis.123456" << EOD
cluster meet 127.0.0.1 16380
cluster meet 127.0.0.1 16381
EOD

sleep 1
redis-cli -p 16379 -a "Redis.123456" cluster addslots {0..5461}
redis-cli -p 16380 -a "Redis.123456" cluster addslots {5462..10922}
redis-cli -p 16381 -a "Redis.123456" cluster addslots {10923..16383}

echo "Redis cluster started."
echo "if fatal, try again."



