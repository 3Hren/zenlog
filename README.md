# Logging Zen

Log transport, filtering and processing.

## Goals
Every library should have a manifest, which describes goals, for which that library was created.

### Use Everywhere
No need to write another swiss-knife-logging-library for every language - it's not scalable! Instead teach it how to dump structured logs over UDP encoded with MessagePack - the rest we will for you.

### Self hosted
