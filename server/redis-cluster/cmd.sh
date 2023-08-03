#!/bin/bash
echo "Starting redis cluster...make sure you have installed redis-server on you host."

rm -rf ~/redis-cluster
lsof -i tcp:16379-16381 | awk 'NR!=1 {print $2}' | xargs kill -9

mkdir ~/redis-cluster

mkdir ~/redis-cluster/16379; mkdir ~/redis-cluster/16380; mkdir ~/redis-cluster/16381

touch ~/redis-cluster/16379/redis.conf; touch ~/redis-cluster/16380/redis.conf; touch ~/redis-cluster/16381/redis.conf

echo "port 16379
cluster-enabled yes
cluster-config-file nodes.conf
cluster-node-timeout 5000
appendonly yes" >> ~/redis-cluster/16379/redis.conf

echo "port 16380
cluster-enabled yes
cluster-config-file nodes.conf
cluster-node-timeout 5000
appendonly yes" >> ~/redis-cluster/16380/redis.conf

echo "port 16381
cluster-enabled yes
cluster-config-file nodes.conf
cluster-node-timeout 5000
appendonly yes" >> ~/redis-cluster/16381/redis.conf

cd ~/redis-cluster/16379 || exit; ls; nohup redis-server redis.conf > /dev/null &
cd ~/redis-cluster/16380 || exit; ls; nohup redis-server redis.conf > /dev/null &
cd ~/redis-cluster/16381 || exit; ls; nohup redis-server redis.conf > /dev/null &

sleep 1s

redis-cli -p 16379 << EOD
cluster meet 127.0.0.1 16380
cluster meet 127.0.0.1 16381
EOD

redis-cli -p 16379 cluster addslots {0..5461}
redis-cli -p 16380 cluster addslots {5462..10922}
redis-cli -p 16381 cluster addslots {10923..16383}

echo "Redis cluster started."
echo "if fatal, try again."



