There are many operating systems.

Even if we narrow it down to operating systems that you can install on most x86_64 computers, there is Windows 11, Linux, and RedoxOS.

Even if we narrow it down to only FOSS operating systems, there is Linux and RedoxOS.

Even if we narrow it down to only operating systems written in Rust, there is RedoxOS.

So why make another OS?

Cuz I want to re-think everything about operating systems. I want to try new things. Imagine the possibilities when you don't really care about commercial use, making breaking changes, or even performance (cuz why spend hours coding to save seconds of execution time?)!

I want to re-think the file system. Do we even need one?

I want to re-think the terminal. Do we even need one?

I want to re-think the way apps draw widgets and forms on the screen. Do we need every app to handle drawing rectangles on the screen?

I want to re-think web browsers. Do we even need one?

And more stuff as I go.

As I started working on this, I realized that it's better to start simple, and then learn what needs to be improved, instead of making a complex thing at first. Cuz I've spent hours on complex things (especially when related to allocators), in the end realizing that it's too messy and not even performant. So a big idea is to start simple and then make things more complex to achieve more performance and flexibility.

I wrote a lot of code. I thought I was re-thinking operating systems the first time I wrote it. But now I'm re-thinking the re-thinking.

I will think about every addition to the kernel, starting with the choice of bootloader.
