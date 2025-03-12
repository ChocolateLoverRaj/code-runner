# Modularity
Some operating systems have a "everything is a file" approach. There are two things I don't like about this. One thing is that you have to deal with file paths such as `/sys/class/power_supply/BAT0/charge_now` to get the mAh of battery left. There are many things I don't like about this:
- The kernel has to represent the battery charge as a file
- A user space process has to:
  - Open the file
  - Read the whole file
  - Parse the string to get the number
  - Close the file descriptor

Instead, it would be so much easier if there was a single syscall which just returned a `u64` value.

Examples of when we want to make things modular:
- You don't want to include frame buffer code in your OS
- You don't want to include serial port code in your OS

So we will need to make syscalls dynamic. Some syscalls may be available and some may not be.

So imagine there are different groups of syscalls: core (always included), A (additional syscalls), B (additional syscalls). Some systems might have just core, some might have core + a, core + b, or core + a + b. The syscall numbers / ids for syscalls should not overlap. We could use a [uuid v4](https://docs.rs/uuid/latest/uuid/) for every syscall. This way if a bunch of people are making their own installable syscalls, there won't be conflicts (assuming everyone randomly generates one).


