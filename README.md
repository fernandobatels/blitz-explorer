# Blitz Archiving Explorer

Service for search and get files inside a multiples .tar.gz files

## Examples

Start the server:

```bash
cargo run /path/to/my/tar/files
```

With server running.

Search the files:

```bash
nc localhost 3355 <<< "/search/my photo.png"
```

The return will be printed in the stdout.

Download a file:

```bash
nc localhost 3355 <<< "/download/photos2018.tar.gz:path/to/my photo.png" > my photo.png
```

The return of /download command is the binary content o file and we redirect him to a local file.
