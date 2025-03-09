## Running
### Quick
```bash
cargo r -- -s -serial stdio
```

### Debugging
```
cargo r -- -s -S -serial stdio
```
In another terminal
```bash
lldb -s debug.lldb
```

### On Real Hardware with iPXE
```bash
cargo r --bin generate_ipxe
```
Then build iPXE with the embed script. Very ez on NixOS. Just do
```bash
./build_ipxe.sh
```
Make sure that your test computer is connected to your development computer with a wired network and it can access the tcp port on your development computer.

Then start the HTTP server:
```bash
cargo r --bin serve_ipxe
```
Then boot `ipxe.efi` on your test computer. What I did on my Chromebook is copied the file to a exfat partition on a USB drive plugged in to my Chromebook. Then in the firmware settings I added a boot option to boot the efi file and set it as the default boot option.

### On a Chromebook
Follow the steps for booting with iPXE, but also connect your Chromebook to the development computer with CCD (I use [my debug board](https://github.com/ChocolateLoverRaj/gsc-debug-board?tab=readme-ov-file#gsc-debug-cable--board) of course). Then make sure that the Chromebook boots `ipxe.efi` by default. Then all you have to do is run:
```bash
cargo r --bin reboot_latest
```
Which will send a reboot command to the Chromebook and also start the server! And you can open `/dev/ttyUSB1` to get logs from the Chromebook! Using this method it takes 22s to boot the OS. This can reduced by reducing the edk2 timeout seconds and can probably be reduced by not enabling serial logging (cuz I think the OS will still be able to do serial logging either way). At least you can disable edk2 serial logging.

### On Real Hardware automatically, through Wi-Fi
I didn't actually do this yet, but this is a theory:
- You need two disks plugged in to the computer you are testing the OS on:
  - A Ventoy
  - A disk with GRUB and a Linux distro with a separate bootloader
- Set up Ventoy to boot the code runner `.img` after 1s
- Add 2 entries to the custom GRUB partition:
  - Chain-load your Linux distro's bootloader
  - Chain-load Ventoy's bootloader
- In your custom GRUB, it should automatically boot the Linux distro's bootloader by default
- In your Linux distro, create a program that starts on boot and either downloads or receives the `.img` from the network. Once it receives the image:
  - Copy the image into the Ventoy folder
  - Run `grub-reboot` so that the next time your custom GRUB boots, it boots Ventoy's bootloader instead
  - Reboot
- In your UEFI firmware settings, set the default boot to the custom GRUB bootloader

Then in your development computer create a program that will:
- Reboot the test computer (which can be done with Chromebooks with CCD by sending `apreset\n` to `/dev/ttyUSB2` (the EC console))
- Send the new image to the Linux distro

With this *super simple* (not really) setup, you can test the latest changes of your code with a single command on your development computer, as long as your test computer has network capability that Linux has drivers for, and you have a network connection (can be Wi-Fi) with your test computer. **No PXE required 🙂**.

## Supported Devices
### QEMU with UEFI
This will have the best support because this is what I mainly test it on.

### Jinlon (HP Elite c1030 Chromebook)
This is the laptop that I use every day. I tried my OS on this laptop. I will make sure it works on it.

### Any Chromebook running UEFI
The nice thing about testing on Chromebooks with [MrChromebox firmware](https://docs.mrchromebox.tech/) is that you can run it on open source firmware and also get serial output.

### Any UEFI x86_64 Computer
It should work. Why not? I didn't try it. But you can. The OS is made mainly for UEFI.

### QEMU with legacy BIOS
I only support this cuz the bootloader supports it and all of the code just works. If this requires any extra effort I will drop support for legacy BIOS cuz I don't have any computers that boot legacy BIOS that I want to run my OS on.

## Guidelines
- Have very flexible code so you can easily modify the OS to be how you want (like u can get rid of the general protection fault handler if you want)
- Reduce power consumption by keeping the CPU halted for as much as we can.
- Don't make a CLI. I will not be making CLI programs for this OS. Most things should be graphical (or terminal) *apps*. For tools which normally benefit from having bash (or other shell), just use a programming language. Write a Rust program. Call some functions. Don't write bash scripts. And since Rust is not as convenient to quickly execute commands maybe I will allow a Python or JavaScript console to use as a CLI. That way at least you get a proper programming language, but with the quick single line commands like a CLI would.
- Avoid preemptive multitasking because I don't see why we would ever need a use for it. This goes along with the power efficiency thing.

## Goals / Progress
The not yet done stuff is in the order that I plan on doing things

- Draw to screen ✅
- Handle keyboard input ✅
- Write to the serial console ✅
- Implement user space and syscalls ✅
- Get time from RTC ✅
- Implement multiple threads and processes 🚧
- Add HPET support 🚧
- Get input from the serial console ❌
- Create a text editor program ❌
- Create a program launcher ❌
- Create a compositor ❌
- Shut down and restart the computer from the OS ❌
- List PCI devices ❌
- Implement USB driver ❌
- Implement user space network driver ❌
- Implement a ping *app* (not command) ❌
- DNS ❌
- TCP ❌
- HTTP ❌
- Load programs on demand through HTTP ❌
