At this point, we
- Set up a pretty, colorful logger
- Set up a simple global allocator
- Set up a useful panic handler
- Set up the IDT and GDT on every CPU
- Have all CPUs ready to do some cool stuff

So let's do some cool stuff. It'll be so cool, we can run code that:
- Does unsafe stuff
- Is potentially malicious
- Can be restarted if it crashes

So we run it in user mode so that we can have fun writing code, minimizing the amount of page faults, undefined behavior, and triple faults that we experience while coding.

I already started simple, with a single user space process. But there are many factors to consider when designing how user space processes run, such as:
- Sandboxing
- Dynamic memory in user mode
- Async in user mode
- Scheduling
- Utilizing all CPUs
- Spawning new processes dynamically
- Logging from user mode

and in this case I don't want to only implement one of those at a time. It should be simple, but powerful.

## Spawning the task
We need to map the ELF and the stack. For this, we need a way of allocating physical frames to certain tasks, and then deleting them when the task is done. We can use the `rangemap` crate to keep track of which physical frames are used for what.

## A simple scheduler
We can start off with a list of tasks, where each task is a user space process. The list is in order of highest priority to lowest priority. The first CPU runs the first task. Then the second CPU runs the next task. If there are more tasks, then CPUs, then the last few tasks will have to wait for the first few tasks to either exit or yield.

1 user. 1 brain. 1 task. We do not need to split attention equally between two tasks. Maybe if there were two users using the same computer at the same time, it would make sense to split CPU execution time between the two, so that both people would at least get to use the computer, instead of one of the users potentially never getting a chance cuz the other user is using 100% CPU.

## A simple message
Of course, we need to say hello world. We need to have user mode code that says hello world so we know the logging is working. For this, we can create a syscall called "log" that takes a `&str` and a `LogLevel`. The kernel will have to make sure that the `&str` is valid. Also, while the kernel is reading the `&str`, the pages that contain the `&str` must not be modified, and it could cause issues if the content is modified, such as invalid utf-8 because a character gets modified across a byte boundary while the kernel is reading it.

At this point, we can demo tasks up to the number of CPUs, which logs messages and continuously uses the CPU. If we have more tasks than the number of CPUs, the extra tasks will never run because the other tasks never give up the CPU.

## A simple exit
Why not make a "exit" syscall? Then we can demo an unlimited number of tasks, each task logging a message and exiting.

## A simple event
We can start off with a single async event: keyboard input. All you have to do to asynchronously receive the event is do a syscall called "Wait until keyboard input". Then the kernel will stop executing the process that did the syscall and return from the syscall when it receives a keyboard interrupt.

### Owning the keyboard?
Does it make sense for multiple processes to have access to the keyboard at the same time? I don't think so. It could lead to a mismatch between the number of interrupts and the number of bytes read from the port. So our kernel needs to keep track of which process, if any, "owns" the keyboard. A process that doesn't own the keyboard is not allowed to read from the keyboard port or do the "Wait until keyboard input" syscall.

Now in addition to tasks that continuously use the CPU until they exit, we can now demo up to 1 (active at a time) task which can run for a long time without using 100% of a CPU.

## Take the Frame Buffer
We want to let programs use the screen. The logger can stop logging to the screen when this happens.

## Interrupts, priorities, and multiple CPUs
The goals:
- Tasks with higher priority should always run first. If the task is waiting for an event, lower priority tasks can run while it waits.
- Preferrably CPUs which don't have a task to run should handle interrupts
- If there are tasks to be done, all CPUs should be utilized

## Storing a list of tasks
Tasks can be identified by id, and they are ordered based on priority level, with higher priority tasks first.

## A proper "Wait until event" syscall
Right now we only have one async event: a keyboard interrupt. Soon we will add HPET interrupts, and other async events such as inter-process communication. The input for this syscall is just `()`. We don't really need to input anything. But for the output we need to tell the user space process which events happened.

### Uniquely identifying events
Some events are fixed, such as a keyboard interrupt. There is exactly one kind of keyboard interrupt event. Not more, not less (well, there wouldn't be an interrupt if there was no PS/2 keyboard for some reason). Some events are unlimited, such as timer events or IPC events. An easy way to identify events would be to use an `enum`. Then unlimited events can have an associated id backed by `usize` or something, and fixed-number don't need an id. We can derive impl `Eq`, so ez.

### Which events happened?
When a process does a "Wait until event" syscall, it needs to know which events happened. That way it knows which `Future`s to `poll`.

We can avoid the problem of the output not fitting in registers by only returning a single event if multiple events happened. Then the user-space process can just call the syscall again after it has processed the first event. This would be a pretty simple solution. However, I can see this being a problem for very high frequency interrupts while the CPU can't keep up. In this scenario, a high-frequency, low-priority event can make it so that higher-priority events are never processed.

For this reason, the kernel will return all events that happened. We can basically have the user space program pass an `&mut [MaybeUninit<EventId>]` to the kernel. Then the kernel can fill the slice with events that happened and return the number of events that happened.

### How is this simple?
The kernel does need to deal with priorities when there are multiple high-frequency interrupts. In the user-space side of the implementation, it can just store a `RefCell<BTreeMap<EventId, AtomicWaker>`, and then create a `Box<[MaybeUninit<EventId>]>` with the size of the number of events.
