# What is dynamically allocated memory?
Everything with a fixed maximum size can use `static`, `const`, and be on the stack. Examples include:

- A global logger variable
- Atomic `bool`s and numbers
- The ram disk

Everything with a dynamic size needs dynamic memory allocation. Examples include:
- Data about user space tasks, when you could have a dynamic number of user space tasks
- Data that grows the longer the OS runs, such as a list of log messages
- Data that is specific to each CPU, and you don't know how many CPUs the OS will run on at compile time
- A very simple program that needs dynamic memory allocation is a text editor. You don't know how much the user will type.

## Why do we need it?
Why can't we just pre-allocate a guess on the maximum memory that will be used?

Well, actually we can. But it will have two issues:

- Computers without enough memory will not be able to even load the kernel
- You can still run out of memory when you exceed the pre-allocated amount

## How static memory works
The ELF file says "This program needs X amount of bytes for static mutable memory". Whatever loads the elf (the bootloader) reserves memory for this, and the code just points to the memory. So whatever memory allocation is needed, the bootloader will handle the memory management.

## How dynamic memory works
When our kernel takes control of the CPU, it knows which physical memory regions are used and which region are available to use (for example with Limine there is the memory map request). The available memory is not necessarily in continuous regions. To actually use this memory, we need to make sure that virtual memory is mapped to the available physical memory. We might need to modify the page tables. We could also use existing offset-mapped virtual memory regions to avoid having to modify page tables.
