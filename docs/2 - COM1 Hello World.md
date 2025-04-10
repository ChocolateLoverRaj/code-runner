We must say "Hello World!". If we don't, how will we know if our code works? Are we going to just assume it works? Are we going to open a debugger and see what line of code is executing?

Hello world is very important. We must show something that indicates our code is running.

We do this by writing to COM1 through the [`uart_16550`](https://crates.io/crates/uart_16550) crate.

I also used the [`log`](https://crates.io/crates/log) crate to easily have macros, and be able to show nice colors for the log level and change the log level filter.

Making "Hello World!" show up on a *virtual machine* is very easy. But how do we do it on Jinlon?

On Jinlon, we have to use memory mapped I/O to read/write to the UART. It uses a 16550-compatible interface, but we have to find out where the address is. We could just hard-code it, but that would be dangerous on other devices, or possibly break after a firmware update. So the proper way is to parse the ACPI tables to find an SPCR table, which tells you how to access the UART.

So right away, we will parse ACPI tables. We will use the very useful [`acpi`](https://crates.io/crates/acpi) for this, without the `alloc` feature because we haven't set up a global allocator yet.
