So we got a [`&acpi::spcr::Spcr`](https://docs.rs/acpi/5.2.0/acpi/spcr/struct.Spcr.html)! From this we can get:
- The interface type
- Base address
- Baud rate

However, on Jinlon, the baud rate is not specified. But we can just use `115200`, which is the correct baud rate. Source: https://docs.mrchromebox.tech/docs/support/debugging.html#suzyqable-debug-cable.

So we can just hard-code the baud rate to `115200` for now. If there is a different device which actually specifies the baud rate in the ACPI table, then we can use that instead, falling back to `115200`.

Now we need to replace COM1 with the MMIO UART when we log stuff. We can use an `enum` to store either a port-based or mmio-based UART in a `static` var for the logger.
