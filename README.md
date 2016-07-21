# Logging Zen

Log transport, filtering and processing.

## Goals
Every library should have a manifest, which describes goals, for which that library was created.

### Use Everywhere
No need to write another swiss-knife-logging-library for every language - it's not scalable! Instead teach it how to dump structured logs over UDP encoded with MessagePack - the rest we will for you.

### Performance
Zen is developing to be the fastest possible solution in its category.

- [ ] Goal - 1kk RPS.
- [ ] Goal - 2kk RPS.
- [ ] Goal - 3kk RPS.
- [ ] Goal - 4kk RPS.
- [ ] Goal - 5kk RPS.

## Protocol
### JSON
Convenience.

### MessagePack
Trade off between convenience and performance.

- [ ] Use RefBox for converting `Vec<u8>` into `ValueRef<'a>`.

### Zen
Maximum performance and safety.

## Signals
Zen runtime understands four type of Unix signals - SIGINT, SIGTERM, SIGHUP and SIGALRM. The rest are handled with the standard system handlers. See POSIX API for details.
