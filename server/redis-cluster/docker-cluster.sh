#!/bin/sh
ADDR1='redis-26380'
ADDR2='redis-26381'
IP1=`ping -c1 ${ADDR1} | sed -nE 's/^PING[^(]+\(([^)]+)\).*/\1/p'`
IP2=`ping -c1 ${ADDR2} | sed -nE 's/^PING[^(]+\(([^)]+)\).*/\1/p'`

# redis-cli not support hostname resolve.
eval "redis-cli -h redis-26379 -p 26379 << EOD
cluster meet ${IP1} 26380
cluster meet ${IP2} 26381
EOD"

redis-cli -h redis-26379 -p 26379 cluster addslots $(seq 0 5461)
redis-cli -h redis-26380 -p 26380 cluster addslots $(seq 5462 10922)
redis-cli -h redis-26381 -p 26381 cluster addslots $(seq 10923 16383)