# Inter-Process Communication
First use case for this:

There are two tasks:
- Main task (a user space program)
- Logging task (as user space program which uses syscalls to access UART)

The main task tells the logging task, "log this string". Then to the main task it is as if the logging is asynchronous, even if the logging task needs to busy-poll to log. Once it is done logging the logging task notifies the main task that it is done logging. So in the main task there would be an `async` function for logging.

So we need to design some syscalls so that any process can send a message to any other process, and then that process can send a message back to the process that messaged it, without being aware of what processes will actually message it. For example, the logging task needs to be able to receive messages from *any other task*. Then it needs to be able to send a message back to the task that asked it to log something. It doesn't make sense to broadcast messages and interrupt every process even if it didn't ask.

## Security
For now we let pretty much any task do anything. Any task can access UART, any task can access the frame buffer, any task can set HPET interrupts. All we are really protecting is user space processes interacting with the hardware in an unsafe or invalid way. But when we design the messaging-between-tasks system we need to make sure that a 3rd process can't access messages between two other processes. Imagine 3 tasks:
- Task A
- Task B
- Logger Task

Both task A and task B can send messages to the logger task, but task B shouldn't be able to access messages that task A sent to the logger task.

One way we could do this is make every task that receives a message have a uuid for the 

## Minimal syscall API needed
We need:
- A destination to send a message to
- Actual message data

### Destination
One way we could do the destination is to have every process have an id, and the destination can just be the id of the process. However, we would need to know the process id of the destination process. We could just make it fixed, but then that would cause problems if you want to run more than one instance of the process. Maybe we could have a *program* id and *process* ids that are separate, so that multiple instances of the same program will have the same program id and different process ids. But still, if there are two processes of the same program and another process sends a messages based on program id, the kernel wouldn't know which process to send the message to.
