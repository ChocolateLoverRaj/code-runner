# Security / Safety Levels
## None
Basically kernel mode. Can access all memory, registers, and instructions and can do invalid stuff.

## User Mode
Can only access memory that the kernel gives it access to. The kernel can give user space access to control things such as the HPET, but the kernel will not let the user space program interact with things in an invalid way. Any user space program can access any syscall.

## User Mode with Permission Restrictions
This is more like Flatpak, Android apps, etc where every program has different permissions. For example, an app could have permission to take the frame buffer but not access to the network, but a server program could have access to the network but not access to take the frame buffer.

The plan is to eventually put most programs in this level of safety / security.
