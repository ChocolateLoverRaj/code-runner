The next step is to set up the IDT and GDT. This needs to be done on every CPU. So let's start* the rest of the CPUs.

*Actually, they are already started, and they are in a busy loop, waiting for an atomic write to the address of the function they should jump to.

On the BSP, we can just call the init CPU function. But before we do that, we need to make sure that existing variables get dropped (such as ACPI tables).

The first thing we need to do in every CPU is to log "Hello, world!" to make sure that it is working.

## Managing Interrupt Numbers
Each CPU can route up to 256 types of interrupts to different interrupt handler functions (in x86). To keep things simple, every CPU will have the same interrupt handler functions. The first 32 interrupt numbers are for x86-defined purposes or they are reserved. The rest of them are free to use however we want. I call it "flexible entries" because they we can decide which numbers to use for them. It doesn't really matter. So far these are the flexible entries we need:
- Local APIC Spurrious Interrupt
- Local APIC Timer
- Local APIC Error
- Keyboard
- HPET (can also have PIT interrupts bundled in because of a limitation of qemu)

The goals:
- Predictable, so that we can configure the I/O APIC to send interrupts to the right number
- No conflicts

Based on these goals, the simplest way of doing this is to just use an `enum` (with `#[repr(u8)]`) and define all of them in a fixed way.

## Logging the CPU id
We should show which CPU logged the message when logging messages. For that, we need to be able to get the current CPU's id from the logger. We can just use a `static` variable for this, because each CPU needs to have its *own* id variable. So instead, we use a `static` variable to store `Box<[SyncUnsafeCell<CpuLocalData>]>`. Then we can set the `GS.Base` register to point to an item in the slice. In the logger, we show "[BSP]" before messages if the CPU local data is not initialized, and we show "[CPU <id>]" before messages if the CPU local data is initialized.

## Updating the panic handler
Before, when we were just running code on 1 CPU, just disabling interrupts and halting the CPU was enough stop the computer from executing any more code. But now, we need to stop every CPU from executing code. We can do this by sending a non-maskable interrupp to all other CPUs through the Local APIC. In the NMI handler, we just halt the CPU. So if a panic happens:
- The CPU that panicked sends a NMI to all other CPUs through the Local APIC
- The other CPUs enter the NMI handler, which halts the CPU
- The current CPU logs the error
- The current CPU halts
