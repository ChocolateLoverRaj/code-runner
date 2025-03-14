# Tasks
Once we enable interrupts, we will no longer be executing code from one line to the next line. We will be jumping around, from user space, to the kernel, to another user space program, to a different thread, etc. We of course want to run more than one thing at once. To do this we will have tasks. Tasks can run as user space (which allows fine-grained permissions) or as the kernel (full access to everything). To increase security (and contain the effects of bugs and reduce the need to reboot the computer), we should minimize kernel tasks and have most tasks in user space.

## User Space Tasks
Each process has its own address space (Cr3 register value). All threads within a process have the same address space. Calling kernel functions require a `syscall` and then the kernel has to do `sysret`. Calling functions in other user space tasks require a `syscall`, Cr3 update, `sysret`, `syscall`, Cr3 update, `sysret`.

## Kernel Tasks
Every kernel task has the same address space (and since we are globally mapping the kernel any Cr3 value will work), but its own stack and heap. Calling kernel functions requires a normal function call (keeping in mind concurrency issues). Calling functions in other kernel tasks does not require `syscall`/`sysret` or Cr3 changes. Kernel tasks should probably not be calling user space tasks because that would be a security flaw and there is probably no good scenario to do it.
