#!/bin/bash

EXECUTE_CMD="$HOME/kafka"
CONFIG_DIR=$(dirname "$PWD")/kafka-cluster

echo $CONFIG_DIR

case $1 in
    "start") {
        KAFKA_CLUSTER_ID=$($EXECUTE_CMD/bin/kafka-storage.sh random-uuid)
        for i in {1..3}
        do
            echo "-------------starting kafka-$i-------------"
            "$EXECUTE_CMD/bin/kafka-server-start.sh -daemon $CONFIG_DIR/server-$i.properties"
            # before first run, you should back commont the next line
            "$EXECUTE_CMD/bin/kafka-storage.sh format -t $KAFKA_CLUSTER_ID -c $CONFIG_DIR/server-$i.properties"
            # $EXECUTE_CMD/bin/kafka-server-start.sh -daemon $CONFIG_DIR/server$i.properties
        done
    };;
    "stop") {
        for i in {1..3}
        do
            echo "-------------stopping kafka-$i-------------"
            $EXECUTE_CMD/bin/kafka-server-stop.sh
        done
    };;
    esac