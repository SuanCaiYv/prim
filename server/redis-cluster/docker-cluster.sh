redis-cli -p 16379 << EOD
cluster meet 127.0.0.1 16380
cluster meet 127.0.0.1 16381
EOD

redis-cli -p 16379 cluster addslots $(seq 0 5461)
redis-cli -p 16380 cluster addslots $(seq 5462 10922)
redis-cli -p 16381 cluster addslots $(seq 10923 16383)