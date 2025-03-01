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

### On Real Hardware
I only ran it on a robo360 (~$45) in case it broke.
```bash
cargo r
```
Then copy the UEFI `.img` file (in my case `/home/rajas/Documents/code-runner/target/debug/build/code-runner-dd2095bbe9ff3898/out/code-runner-uefi.img`) to a Ventoy.


### On Real Hardware automatically, without Plugging or Unplugging anything
I didn't actually do this yet, but I will eventually:
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
