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

We can basically get a `&'static mut [u8]`. Then we have a lot of options for allocators. We can use [`linked_list_allocator`](https://crates.io/crates/linked_list_allocator), or one of the many allocators on crates.io.

## Running out of memory for the global allocator
One potential issue is the global allocator running out of memory. Since if it is an allocator backed by `&mut [u8]`, we will have to guess how much memory we'll need when we initialize it. Then it might run out. In this case, it's best to have a way of mapping more pages to add onto the memory that the global allocator can use. What makes this complicated is that the `invlpg` instruction needs to be called on every CPU. So if one CPU adds pages to the global allocator, all other CPUs need to call `invlpg` on those pages *before* they access the memory. And since we don't know which common data requires which `invlpg`, we will have to basically interrupt all of the other CPUs and make them do `invlpg` right away.

## Locking and Interrupts
If we have a locked allocator, then it could cause problems if an interrupt happens during an allocation and the interrupt handler tries to allocate. In this scenario the OS would gets stuck since the interrupt handler would spin forever. The easy solution is to never allocate in interrupt handlers. A more advanced solution is to use a lock-free allocator.

## Locking and multiple CPUs
When we run code on multiple CPUs, it is possible that more than 1 CPU tries to allocate at the same time. In this scenario, the other CPUs will have to wait for the first CPU to finish allocating before they can continue. This will decrease performance. We may not get the full performance benefit of having multiple CPUs. One way to solve this is to use separate allocators for each CPU when they are dealing with CPU-specific data. Each CPU can have some memory pre-allocated to it and then allocate more memory to its CPU-specific pool if it runs out.

## A simple start
Before, I spent a long time making fancy solutions, such as an allocator which modifies page tables basically every time it allocates (and deallocates). This ended up having bugs and it was frustrating, and I'm sure modifying page tables and calling `invlpg` on every allocation is not good for performance. *And* I would have to also tell all other CPUs to do `invlpg` on every allocation.

So this time, I will start very simple, and have a `static`-backed global allocator. This way we don't need to find virtual and physical memory to set up the global allocator. In fact, this allocator is so simple that I could just set it up before setting up logging. Then we could immediately start using `Box` and `String` for things. I will use `linked_list_allocator` since I've used it before and that's what is on https://os.phil-opp.com.
