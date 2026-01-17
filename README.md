# xkcdfs-fuse

A read-only FUSE filesystem that exposes the latest XKCD comic as files.

## Features

*   Fetches the latest XKCD comic metadata and image on startup.
*   Exposes `latest/title.txt`, `latest/alt.txt`, and `latest/image.png`.
*   Includes a desktop entry and an about file.
*   Supports running in the foreground or as a daemon.

## Usage

### Build

```bash
cargo build --release
```

### Run

Create a directory to serve as the mountpoint:

```bash
mkdir <mountpoint>
```

Run the filesystem (as a daemon):

```bash
./target/release/xkcdfs-fuse --mountpoint <mountpoint>
```

To run in the foreground:

```bash
./target/release/xkcdfs-fuse -- -f <mountpoint>
```

### Unmount

```bash
fusermount -u <mountpoint>
```