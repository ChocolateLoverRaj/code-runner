I heard that some old computers have a thing called "Legacy BIOS" which is older and not as easy to boot from from UEFI. And QEMU by default has legacy BIOS. But like, we don't need to support every boot mechanism. We can start simple, targeting one boot mechanism. My test computer is a modern laptop. It is a Chromebook. Here are the specs:

- Board name: Jinlon
- Processor: Intel Core i7-10610U
- RAM: 16GB Soldered
- Storage: 1TB NVMe SSD (upgradable)
- It has a touchscreen display.
- It has a keyboard and I'm not sure if it's actually a PS/2 keyboard, but it can be used as a PS/2 keyboard by the kernel.
- It has Chromebook speakers.
- It has a ChromeOS Embedded Controller (which I think will be cool to send commands to because there is open source code which interacts with it)

I do want to make an OS that will run on real computers. It's not fun if it only runs in a virtual machine. Of course, it has to run on a virtual machine. It would be way too hard to test if it didn't.

So we have two targets:
- QEMU with UEFI
- Jinlon running MrChromebox firmware (UEFI)

And the way we can make the computer boot our OS is by telling the UEFI firmware to boot from a `.efi` file. The firmware can do things like add boot options to boot from a specific `.efi` on a FAT partition, boot from the default `.efi` file on a storage device, and chain-boot from another UEFI application.

We could write our own bootloader. Or use an existing one such as:
- https://github.com/rust-osdev/bootloader
- [Limine](https://github.com/limine-bootloader/limine)

## Rust osdev bootloader
We make our initial choice the simplest option. IMO the simplest option is the Rust osdev bootloader. It makes it very easy to run the OS in QEMU and also pretty easy to run on real hardware.

## Limine
However, I switched to using Limine instead, because it makes it very easy to run code on all of the CPUs instead of just one. Some people have modified the Rust osdev bootloader to work with multiple CPUs, but it is not maintained.

Another advantage to using Limine is that it supports many architectures, not just `x86_64`. This will make it more *simple* to add support for other architectures to this OS.

Also, Limine looks clean, and focused. Limine is not just a bootloader, it's a bootloader *protocol*. In the docs it says:
> The Limine boot protocol is a modern, portable, featureful, and extensible boot protocol.

In the FAQ is states clear goal of providing only what's necessary, and not being bloated like GRUB2. Limine is modern, and I think it could be the new standard (maybe for Arch users only though).
