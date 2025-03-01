# Entering User Space
## GDT Setup
- We create the following segments in the GDT (order matters):
```rs
let mut gdt = GlobalDescriptorTable::new();
let kernel_code_selector = gdt.append(Descriptor::kernel_code_segment());
let kernel_data_selector = gdt.append(Descriptor::kernel_data_segment());
let tss_selector = gdt.append(Descriptor::tss_segment(tss));
let user_data_selector = gdt.append(Descriptor::user_data_segment());
let user_code_selector = gdt.append(Descriptor::user_code_segment());
```

- We set the following registers when loading the GDT:
```rs
CS::set_reg(self.kernel_code_selector);
SS::set_reg(self.kernel_data_selector);
DS::set_reg(SegmentSelector::NULL);
ES::set_reg(SegmentSelector::NULL);
```

## Before entering user space
- We write to the `STAR` register a value based on the segments. The `x86_64::registers::model_specific::x86_64::Star::write` method and it does the calculations for us.
- We set the `EFER` MSR to enable the `syscall` instruction in user space
- We set the `FMASK` MSR to disable interrupts when the CPU enters the syscall handler.
- We set the `LSTAR` MSR to the virtual address of the syscall handler function.

## Loading the user space code
### Just enter a kernel's function
We *could* just enter some function in our kernel which runs as user space, but this can cause many problems:
- The user space program can read the kernel's code. If your kernel code is not a secret then this is okay.
- The user space program can't access any `static` variables because it wouldn't be allowed to (if you did allow it then there is basically no point in entering user mode because you're letting the user space program access and mess up the kernel's memory)
- It would be messy when you have a lot of user space code and you accidentally call function that user mode is not allowed to call, such as enabling/disabling interrupts from user mode.

### Load an ELF
The more proper solution is to have a separate Rust (bin) crate for the user space program. ELF makes it easy to load a program into memory and correctly set the page permissions such as read, write and execute for different parts of the memory that make up the user space program.

So basically we need to:
- Put every segment of the ELF into memory (doesn't need to be in a fixed spot) with the correct flags (don't forget the user accessible flag)
- Put a user accessible stack somewhere in memory (stack size just needs to be big enough to not have a stack overflow in the user space program)
- Enter user space

## Entering user space
We just need to know what instruction to jump to and what the end of the stack should be. In a higher level this means we need to know the address of the entry function in the user space program and the user space program's stack.
- Set the `r11` register to the `EFLAGS` register value we want. In this case, we want to enable interrupt so that we don't get interrupted before entering user space but as soon as we enter user space interrupts are enabled.
- Set `rcx` to the address to jump to
- Set `rsp` to the rsp we want the user space program to have. Of course don't do any instructions that involve the stack between this and `sysretq` because that will cause undefined behavior and might result in the `rsp` value being different than what's expected.
- Do the `sysretq` instruction to enter user space.


# Syscalls
## Inputs and Outputs
### `syscall` instruction's calling convention
The registers `rdi`, `rsi`, `rdx`, `r10`, `r8`, `r9`, and `rax` can be set by user space before `syscall` and then accessed by the syscall handler. Then `rax` is set by the syscall handler and read by user space. 

I am not sure why more registers aren't used as inputs and outputs. It seems to be fore performance reasons. Linux uses the 7 registers as input and 1 output as mentioned above (althought it says it has 6 inputs and `rax` is the syscall number).

### `sysv64` calling convention
Based on [OSDev](https://wiki.osdev.org/System_V_ABI#x86-64):
> Parameters to functions are passed in the registers rdi, rsi, rdx, rcx, r8, r9, and further values are passed on the stack in reverse order.

> The return value is stored in the rax register

### Converting between `syscall` and `sysv64` "calling conventions"
I don't think `syscall` is actually a calling convention, since you can make it any way you want. But we still need to end up calling a rust function (which uses the `sysv64` calling convention) from user space. The following registers are used for both `syscall` and `sysv64`:
- `rdi`
- `rsi`
- `rdx`
- `r8`
- `r9`

`sysv64` uses `rcx` as an input, but we can't set `rcx` before the `syscall` instruction because the `syscall` instruction modifies `rcx` internally. So we set `rcx` to the value of `r10` between the user space and the Rust syscall handler. We push `rax` onto the stack as the 7th parameter.

## Security
Because `syscall` does not switch stacks, the syscall handler runs on the user space stack. After returning back to user space, the kernel's internals could be accessed by the user space program. This is why we switch to a stack that only the kernel can access during the syscall handling.

It might be necessary to zero some registers before `sysret`ing too, but idk.

# Running `async` in user space
Without `async`, why bother trying to do two things at once in your code? It's possible, but imo it's not fun. 

Here are some things we can and can't (funly) do in our OS:

## Doesn't need async
```rs
loop {
    // Do something
    sleep_ms(1000);
}
```
```rs
loop {
    let keyboard_input = get_keyboard_input();
    // Do something with the input
}
```

## Needs async
```rs
let keyboard_future = get_keyboard_input_async();
let timeout_future = sleep_ms_async(1000);

let what_happened_first = futures::future::select(keyboard_future, timeout_future);
```

## How the CPU does async stuff
- CPU is running some code (let's call it code A)
- Interrupt happens and CPU jumps to the interrupt handler, giving the interrupt handler information on what it was previously doing (code A)
- Interrupt handler does stuff
- Interrupt handler can exit and tell the CPU to continue where it left off by `iretq`ing with the information about where the CPU was before
- CPU continues running code A

## General way of waiting for something to happen and not spinning
- Halt the CPU and enable interrupts
- The interrupt handler gets called and updates some memory to indicate that it was called + maybe some data it got (such as a keyboard scan code)
- The CPU "finishes" the `hlt` instruction and the next line of code runs
- The code can check if the memory was modified and indicates that the async thing happened. If it did happen it can do whatever it does. Otherwise it can just halt the CPU and enable interrupts.

## Executor in the kernel
This is my quick overview on how [Philipp Oppermann's blog *Writing an OS in Rust*](https://os.phil-opp.com/async-await/) implements executing `async` in the kernel (assuming the executor only executes a single future and then returns when the future is done):
- The executor polls the future
- When the future is polled, it sets up an interrupt so that the interrupt handler will wake the waker when it's interrupted. It returns `Poll::Pending`
- Obviously if it's ready then the executor just returns the value.
- The executor enables interrupts and `hlt`s.
- The interrupt handler gets called. The interrupt handler updates some memory to indicate that the event happened.
- The executor's `hlt` "function" "returns"
- The executor polls the future again
- When the future is polled, it returns `Poll::Ready` because it reads the memory that the interrupt handler updated and knows that the event happened
- The executor is done

## Why the executor in the kernel won't work in user space
- You can't `hlt` or enable/disable interrupts in user space.
- You can't directly have an interrupt handler in user space. Even if the interrupt handler was set to run in user space (ring 3), this doesn't really make sense since the interrupt handler itself probably needs to do things that require kernel privilege (such as reading data from the keyboard and interacting with the HPET)

## Very low level explanation of `async` in Rust
There is the `std::future::Future` trait which has a `poll` method. When calling a poll method you input a `std::task::Context` which is basically is a way of the future waking up the executor.

The executor takes a `std::future::Future` and repeatedly polls it. A useful executor sleeps and wakes up when a future is ready to be polled again instead of continuously busy polling. This way no CPU time is wasted and the code is still very responsive.

## What we need for `async` in Rust
- A way of "sleeping" such as the `hlt` instruction. The async functions / futures don't need to do this part themselves. The executor is what sleeps.
- A way of waking a waker that is independent of the executor. The way that the waker gets woken up cannot depend on the executor.

## Designing syscalls to make `async` possible in user space code
The only idea I can think of is:
- The kernel's interrupt handler calls a user space function, which is basically an interrupt handler but not directly called by the CPU.
- Have a syscall that is similar to `hlt`, but isn't necessarily just a wrapper around the `hlt` instruction. The syscall can "block" execution of the user space program. After it does that, the kernel can do whatever it wants. It can run other user space programs, run kernel tasks, or `hlt`.

## Interrupt handlers vs threads in user space
### What does it mean to be an "interrupt handler"?
When I think of it, I imagine the main code / `fn main() {}` to be "interrupted". The main function's execution is paused, the interrupt handler runs, and then the main function's execution is resumed. But maybe if there were multiple CPUs and the kernel supported having multiple CPUs, we wouldn't actually have to pause the execution of the main function. We could just run the interrupt handler in parallel because anyways Rust doesn't make any safety assumptions about this (besides `Send` and `Sync`, but we can just always require the interrupt handler to be `Send` and `Sync`, so that won't really matter).

### What does it mean to run code in parallel
To a user space program, many things could be seen as happening "in parallel" whether or not they are actually running on multiple cores at the same time. There could be multiple scenarios that affect how real time changes but the code execution is not 1:1:
- The CPU itself isn't always consistent in how fast it runs code
- The kernel can pause execution of a program whenever it wants and then resume it later whenever it wants
- The kernel can share CPU time between multiple programs so there will be small time gaps in code execution
- An interrupt handler happens, and the interrupt handler runs "in parallel" to the main code but in reality only 1 of them run at the same time.

### Conclusion
When we design an API for user space processes to interact with the kernel, we should treat interrupt handlers as running "in parallel" so we can do cool things like run the interrupt handler at the same time as the main code run multiple interrupt handlers in parallel, and interrupt the interrupt handler and run a higher priority interrupt handler.

## Priority of interrupt handlers
Especially when there is only 1 CPU, we will have times when two parts of code want to run at the same time, but they physically can't. We should hopefully explicitly prioritize certain things so that things work nicely. For example, a game shouldn't have missed keyboard input. 

### We need a way for the user space program to specify the priority of one interrupt handler over the other
Imagine two interrupts: a timer, and a keyboard. A program might set a timer, and do something in the timer interrupt. And it might have a "press escape to cancel" feature. So in this case, we would want the keyboard interrupt to have higher priority than the timer interrupt (in the case of a single core CPU it means the keyboard interrupt interrupt the timer interrupt if the timer interrupt was executing).

But then imagine if you had a game where you only had a certain amount of time to finish a level. Then the timer interrupt should interrupt the keyboard interrupt.

### Does it even matter?
In our async executor, the interrupt handlers themselves don't do anything besides waking the executor. There shouldn't be a problem with an interrupt handler blocking another interrupt handler from executing.

### Handling vs processing
When we use interrupt for async Rust, we actually don't *process* the interrupt (such as a timer or keyboard input) in the interrupt handler function itself. We just do the minimum interaction with the device necessary (such as setting up the next interrupt on the HPET or reading from the PS/2 keyboard) and then let the main function actually do stuff with the knowledge that the interrupt happened.

### Should user space interrupt be disabled at all times except for when the executor is sleeping?
Imagine the Maze Roller Game:
- The whole game is a future
- The executor calls `poll` on the future. The future then checks if there was a keyboard press, the game moves the ball and redraws the game on the screen.
- After it redraws the game on the screen, the executor sleeps, unless an interrupt happened while `poll` was called.

There is no advantage to having the interrupt handlers get called while the game is doing it's logic and drawing. The keyboard input is not going to get processed any sooner. It'll just slow down the rendering (maybe) because the drawing will be interrupted. So maybe, the executor should only enable interrupt when it is going to sleep.

### What if we actually want to get interrupted?
There are use cases where a program will want to do a CPU/blocking intensive task such as render to the frame buffer, and when we receive an event such as a timer interrupt or keyboard input we want to pause doing the CPU intensive task and so something else, and maybe stop the CPU intensive task completely. We would want to call a synchronous function which doesn't willingly give up CPU time (because it doesn't stop being busy with things to do) and have the kernel return control back to our main function when there is an interrupt. This is called threads.

### How many interrupt handlers to run?
Imagine a program which does some CPU stuff, and in that time, it receives a keyboard interrupt and timer interrupt. Then when the executor is ready to wait for interrupts again and tells the kernel to enable interrupts and not return control back to the main function until an interrupt is received? What should happen if there are multiple interrupt handlers waiting to be called? Should just one be called? Should all pending interrupt handlers be called?

I think all interrupt handlers should be called. If only 1 was called then it's possible that other interrupt handlers would never be called if the main function blocked for long enough between checking for interrupts.

### Does the order of interrupt matter?
Well it doesn't really matter because interrupt handlers will be really short anyways. However what if the interrupt handlers took so long that they were never done?

Here is my idea for this:
- When interrupts need to be called, take a snapshot of all the currently pending interrupts.
- "Call" all of the interrupts
- Return control to the user space's main function

This way there isn't a scenario where the interrupt handlers are being called so often that the main function never gets a turn.

### How will creating threads work?
- There can a syscall to create a thread
- Threads can exit by calling a syscall
- The code that spawned the thread can also delete a thread (if that code gets control of the CPU)

### Threads and prioritization
#### main thread vs created thread
Let's say we have two threads:
- The main function gets called
- It creates a thread

Let's just call them "main thread" and "created thread".

Which thread should have priority?

I think it should always be the main thread, because every use for another thread is to offload CPU intensive tasks to another thread so that the main thread can remain responsive to things like timers and keyboard input and take control from the created thread.

#### created thread 0 vs created thread 1
Let's say we have 3 threads:
- The main function gets called
- It creates a thread
- It creates another thread

Let's call them "main thread", "created thread 0", and "created thread 1".

The main thread should have more priority than the two created threads. What should be prioritized, "created thread 0" or "created thread 1"? I have some ideas on what our options are:
- Let the user space program tell the kernel which thread to prioritize
- Always prioritize the first thread created
- Make no guarantees about which one is prioritized, and the kernel can just run whichever one is more convenient

If there are any scenarios where we need to specify which thread should have higher priority, then we should definitely implement specifying the priority. Here are some scenarios:
- One thread for rendering, one thread for logging. The logging thread should have lower priority.
- One thread for rendering, one thread for doing math calculations. The rendering thread should have higher priority.
- One example with 3 threads (in order of highest to lowest priority): rendering, logging, math calculations.

#### APIs for creating threads with prioritization
We could just specify a priority number (can be a `u64` for high flexibility). This would be very easy to implement in the kernel but when writing programs it can be hard, especially when the number of threads is dynamic.

We could also specify relative priority compared to other threads. This would definitely be harder to implement in the kernel. This might make user space programs easier, but also it would create complication if the relative thread was deleted.

I think for now we should just have a priority number. Better to start simple and then make things more advanced when we encounter scenarios that require more advanced scheduling than to start off with a complicated scheduling method and then realize that it doesn't really work. 

### The Plan
- Create a syscall such as `enable_interrupts_and_block_until_interrupt_received_and_process_all_interrupts_and_then_disable_interrupts`. This name is too long (imo) so we need to find a better name for this.
- Create a syscall for exiting from a user space interrupt handler.
- Figure out how multiple threads and processes will work.
- Figure out if the kernel should have a different Cr3 than user space programs instead of mapping the kernel memory to the address space of every user space program (for security reasons).
- Figure out how to run stuff on multiple CPUs.
- Figure out how prioritization will work when there are multiple threads and processes.
