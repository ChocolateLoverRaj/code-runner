# Logging
All tasks need logging (at least when debugging). We will not have a `stdin`, `stdout`, or `stderr` in this OS. Logging can be done through optional syscalls. How it will work is that each process will have basically a `Vec` of log messages, which consist of a log level (we'll just use the levels that the `log` crate has) and a message string. That way if you are debugging through the console you have the flexibility of just dumping all logs, or getting logs for a specific task based on log level.

For now, to keep things simple, we will have an unbounded `Vec` for log messages, so logging can be synchronous and instantaneous and reading what has been logged can also be without locking and instantaneous.

## Syscalls for writing to logs
- Log(log level, message str)

## Syscalls for reading logs
Reading current logs is ez pez. But what if we want to watch as logs happen and continuously display the latest logs? We need some way of asynchronously receiving log messages. For this we will need a task (which can run as kernel) to receive log messages.
