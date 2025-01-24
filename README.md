## Running
### Quick
```bash
cargo r -- -s -serial stdio
```

### Debugging
```
cargo r -- -s -S -serial stdio
```
In another terminal
```bash
lldb -s debug.lldb
```

### On Real Hardware
I only ran it on a robo360 (~$45) in case it broke.
```bash
cargo r
```
Then copy the UEFI `.img` file (in my case `/home/rajas/Documents/code-runner/target/debug/build/code-runner-dd2095bbe9ff3898/out/code-runner-uefi.img`) to a Ventoy.
