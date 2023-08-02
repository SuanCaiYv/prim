#!/bin/zsh
# type your http proxy server address
# the default value is local host.

# Function to get local IP address
get_local_ip() {
    if [[ "$(uname)" == "Darwin" ]]; then
        # macOS
        local_ip=$(ifconfig | grep -Eo '(addr:)?([0-9]*\.){3}[0-9]*' | grep -v '127.0.0.1' | head -n 1)
    else
        # Linux
        local_ip=$(hostname -I | awk '{print $1}')
    fi
    echo "$local_ip:7890"
}

# Prompt user for input
read -p "Enter IP address (press Enter for default local IP): " user_input

# Use local IP if user_input is empty
if [[ -z "$user_input" ]]; then
    user_input=$(get_local_ip)
fi

proxy_address="http://${user_input}"

docker build --build-arg http_proxy=$proxy_address --build-arg https_proxy=$proxy_address -t prim/scheduler-amd64-linux -f ../docker/amd64-linux/dockerfile-scheduler . &&
docker build --build-arg http_proxy=$proxy_address --build-arg https_proxy=$proxy_address -t prim/message-amd64-linux -f ../docker/amd64-linux/dockerfile-message . &&
docker build --build-arg http_proxy=$proxy_address --build-arg https_proxy=$proxy_address -t prim/seqnum-amd64-linux -f ../docker/amd64-linux/dockerfile-seqnum . &&
docker build --build-arg http_proxy=$proxy_address --build-arg https_proxy=$proxy_address -t prim/api-amd64-linux -f ../docker/amd64-linux/dockerfile-api . &&

docker tag prim/scheduler-amd64-linux ghcr.io/suancaiyv/prim/scheduler-amd64-linux:latest &&
docker tag prim/message-amd64-linux ghcr.io/suancaiyv/prim/message-amd64-linux:latest &&
docker tag prim/seqnum-amd64-linux ghcr.io/suancaiyv/prim/seqnum-amd64-linux:latest &&
docker tag prim/api-amd64-linux ghcr.io/suancaiyv/prim/api-amd64-linux:latest &&