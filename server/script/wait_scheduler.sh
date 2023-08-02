#!/bin/bash

MAX_ATTEMPTS=5
IP="scheduler.prim"
PORT="11222"

echo "Checking service availability at $IP:$PORT..."

for ((attempt=1; attempt<=$MAX_ATTEMPTS; attempt++)); do
  if nc -zv $IP $PORT; then
    echo "Service is available at $IP:$PORT."
    break
  else
    echo "Service not available. Retrying in 1 second..."
    sleep 1
  fi
done

if [ $attempt -gt $MAX_ATTEMPTS ]; then
  echo "Service is still not available after $MAX_ATTEMPTS attempts."
fi
