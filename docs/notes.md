# Start of syscall handler
0xffff80000006c518

# before switching stacks

# sysretq
0xffff80000006c568

# handle_syscall
handle_syscall

# Registers that should be preserved
register read rsp rcx r11 rbp rbx r12 r13 r14 r15

# To watch a u64
w s e -- 0x0000600000794618

rsp before switching to temp stack: 0x0000000000042648
new rsp to set:                     0xFFFF8000000B3CD0
rsp in closure:                     0xffff8000000b3410
rsp after closure:                  0x0000000000042648
