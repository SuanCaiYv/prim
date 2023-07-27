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

attention!

if you are using OrbStack as your docker engine front-end,
please exit it and turn Docker-Desktop(for macOS and Windows) on.
using OrbStack for launch this service need some additional work to be done,
and it's hard to configure.