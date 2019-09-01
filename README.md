# Json FUSE FS

## Build requirements

To build, you need to have installed both fuse headers and openssl headers. In Fedora:

```bash
dnf install fuse-devel openssl-devel
```

## Run

To mount a json descriptor as a file system, run:

```bash
cargo run [json_descriptor] [mount_directory]
```

You can configure `RUST_LOG` env variable to increase log level verbosity

To unmount **don't kill the application**. Run:

```bash
fusermount -u [mount_directory]
```