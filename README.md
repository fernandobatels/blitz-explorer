# Blitz Archiving Explorer

Service for search and get files inside a multiples .tar.gz files

## TCP Server

Search and download the indexed files inside of yours tar.gz files connection on the TCP server.

The protocol basically have two only commands: /search and /download

Start the server:

```bash
cargo run /path/to/my/tar/files
```

Search the files (/search/FILE NAME HERE):

```bash
nc localhost 3355 <<< "/search/my photo.png"
```

The return will be printed in the stdout.

Download a file (/download/you compressed file.tar.gz:PATH/TO/FILE.png):

```bash
nc localhost 3355 <<< "/download/photos2018.tar.gz:path/to/my photo.png" > my photo.png
```

The return of /download command is the binary content o file and we redirect him to a local file.

## FileSystem interface(With fuse)

For more interactivity you can use a mounted file system to consume yours indexed files:

```bash
mkdir /mnt/mytars
cargo run /path/to/my/tar/files /mnt/mytars
```

Now you can use File Manager(like the Dolphin) for access yours files or just use the ls, cp...

To unmount your "tars" file system:

```bash
fusermount -u /tmp/aqui9
```

