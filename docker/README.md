follow this [one](https://github.com/cross-rs/cross/wiki/Getting-Started) to install cross-rs.

pull docker image manually by:

``` bash
docker pull --platform linux/x86_64/v8 ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5
```

for amd64

and

``` bash
docker pull --platform linux/x86_64/v8 ghcr.io/cross-rs/aarch64-unknown-linux-gnu:0.2.5
```

for aarch64