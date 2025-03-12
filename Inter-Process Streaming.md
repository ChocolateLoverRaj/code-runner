# Inter-Process Streaming
Example scenario's tasks:
- Main Task
- Logging Task

The main task wants to send a stream of characters to be logged.

## Memory layout
The two tasks will share memory, which will need to be a multiple of a page size. Here is how it will be:
```rs
struct SharedMemory<const N: usize> {
    /// Modified by writer, read by reader
    write_position: AtomicUsize,
    /// Modified by reader, read by writer
    read_position: AtomicUsize,
    /// Read by both writer and reader
    /// Writer changes this from `false` to `true`
    /// Reader changes this from `true` to `false`
    write_looped_around: AtomicBool,
    /// Modified by writer, read by reader
    data: [MaybeUninit<u8>; N],
}
```

## Writing to the stream
- Find out how much free space there is in the buffer
  - Read `write_looped_around`, `write_position`, and `read_position`
    - If `write_looped_around` is `false`, then the amount of free space is `(N - write_position) + (read_position - 0)`
    - If `write_looped_around` is `true`, then the amount of free space is `read_position - write_position`
- If there is free space, write the amount of bytes you want, starting at `write_position`, looping around once you reach the end of the buffer.
- If you looped around, set `write_looped_around` to `true`
- Update `write_position` to the new position

## Reading from the stream
- Read `write_looped_around`, `write_position`, and `read_position`
- Read / process the written data which has not been read yet
- If you looped around, set `write_looped_around` to `false`
- Update `read_position` to the new position

## Synchronization Issues
(Correct me if I'm wrong) this method should have no synchronization issues because if the reader and the writer are accessing the data at the same time, the worst thing that will happen is the reader will not read the full amount available to read, or the writer will not write to the full amount. `write_looped_around` will never be set to an invalid value because the reader only changes from `true` to `false` and the writer only changes from `false` to `true`.
