FROM scratch
WORKDIR /prim
RUN cd /prim && mkdir server && mkdir client
COPY ./target/x86_64-unknown-linux-musl/release/scheduler /prim/server/scheduler
COPY ./target/x86_64-unknown-linux-musl/release/recorder /prim/server/recorder
COPY ./target/x86_64-unknown-linux-musl/release/api /prim/server/api
COPY ./target/x86_64-unknown-linux-musl/release/message /prim/server/message
