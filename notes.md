# Start of syscall handler
0xffff8000000564e0

# before switching stacks
0xffff80000005650d

# sysretq
0xffff80000005652f

# handle_syscall
handle_syscall

# Registers that should be preserved
register read rsp rcx r11 rbp rbx r12 r13 r14 r15

# To watch a u64
w s e -- 0x0000600000794618
