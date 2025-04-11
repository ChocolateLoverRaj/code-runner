## Why
We at least need to log the panic message during a panic handler so that we know why the OS stopped without a debugger. Especially for panics that happen on real computers.

## Disabling Interrupts
We don't want to get an interrupt after a panic occurs because at this state, the OS is in an undefined state. We need to halt immediately and minimize the amount of code that runs after the panic. So immediately we disable interrupts in the panic handler.

## Implementation
We may not be able to do `log::error!()` if the panic handler happened while the logger was locked. If we do, then will might spin forever.

One solution is to just have a separate logger during panic handlers. This will have the advantage of having consistent not-corrupted logging because the logger will start with a fresh state.

Another solution is to force-unlock the logger before logging the panic message. This will avoid the deadlock scenario, but it could cause corrupted messages since we will be re-calling log functions while their state could be in the middle of logging the previous message. This is the simplest solution, but it could break depending on how the logging works internally. It could be as simple as adding `\n` before the panic message.

## A simple start
I'm going to use the force-unlock method because it's simpler to implement. It's pretty hacky, but we can change it later.

## Make it Fancy?
What if we drew a blue screen with a frowny face and a QR code like on Windows 11? This would be cool, but I'm not going to do it. Because the system is an undefined state, doing fancy stuff like that might cause even more issues. Also, I would rather spend time making sure the kernel *doesn't* panic in the first place rather than making the panic handler fancy.
