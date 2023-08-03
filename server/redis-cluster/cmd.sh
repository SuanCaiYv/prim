#!/bin/zsh
echo "Starting redis cluster...make sure you have installed redis-server on you host."

lsof -i tcp:16379-16381 | awk 'NR!=1 {print $2}' | xargs kill -9

rm -rf ./16379; rm -rf ./16380; rm -rf ./16381

mkdir -p ./16379; mkdir -p ./16380; mkdir -p ./16381

touch ./16379/redis.conf; touch ./16380/redis.conf; touch ./16381/redis.conf

echo "port 16379
cluster-enabled yes
cluster-config-file ./16379/nodes.conf
cluster-node-timeout 5000
appendonly yes" >> ./16379/redis.conf

echo "port 16380
cluster-enabled yes
cluster-config-file ./16380/nodes.conf
cluster-node-timeout 5000
appendonly yes" >> ./16380/redis.conf

echo "port 16381
cluster-enabled yes
cluster-config-file ./16381/nodes.conf
cluster-node-timeout 5000
appendonly yes" >> ./16381/redis.conf

nohup redis-server ./16379/redis.conf > /dev/null &
nohup redis-server ./16380/redis.conf > /dev/null &
nohup redis-server ./16381/redis.conf > /dev/null &

sleep 3

redis-cli -p 16379 << EOD
cluster meet 127.0.0.1 16380
cluster meet 127.0.0.1 16381
EOD

redis-cli -p 16379 cluster addslots {0..5461}
redis-cli -p 16380 cluster addslots {5462..10922}
redis-cli -p 16381 cluster addslots {10923..16383}

echo "Redis cluster started."
echo "if fatal, try again."



