# Inputs and Outputs
## `syscall` instruction's calling convention
The registers `rdi`, `rsi`, `rdx`, `r10`, `r8`, `r9`, and `rax` can be set by user space before `syscall` and then accessed by the syscall handler. Then `rax` is set by the syscall handler and read by user space. 

I am not sure why more registers aren't used as inputs and outputs. It seems to be fore performance reasons. Linux uses the 7 registers as input and 1 output as mentioned above (althought it says it has 6 inputs and `rax` is the syscall number).

## `sysv64` calling convention
Based on [OSDev](https://wiki.osdev.org/System_V_ABI#x86-64):
> Parameters to functions are passed in the registers rdi, rsi, rdx, rcx, r8, r9, and further values are passed on the stack in reverse order.

> The return value is stored in the rax register

## Converting between `syscall` and `sysv64` "calling conventions"
I don't think `syscall` is actually a calling convention, since you can make it any way you want. But we still need to end up calling a rust function (which uses the `sysv64` calling convention) from user space. The following registers are used for both `syscall` and `sysv64`:
- `rdi`
- `rsi`
- `rdx`
- `r8`
- `r9`

`sysv64` uses `rcx` as an input, but we can't set `rcx` before the `syscall` instruction because the `syscall` instruction modifies `rcx` internally. So we set `rcx` to the value of `r10` between the user space and the Rust syscall handler. We push `rax` onto the stack as the 7th parameter.

# Security
Because `syscall` does not switch stacks, the syscall handler runs on the user space stack. After returning back to user space, the kernel's internals could be accessed by the user space program. This is why we switch to a stack that only the kernel can access during the syscall handling.

It might be necessary to zero some registers before `sysret`ing too, but idk.
