upatch: Live-patch in userspace

## Introduction

We implement a live-patch mechanism in userspace based on UPROBE.
We have made a presentation in OSSEU, check following links for more information about this project:
+ [LinuxCon Europe 2022 - Livepatch in Userspace Based on Uprobe - PDF](https://static.sched.com/hosted_files/osseu2022/19/OSS-EU22-Livepatch-in-Userspace.pdf)
+ [LinuxCon Europe 2022 - Livepatch in Userspace Based on Uprobe - Video](https://www.youtube.com/watch?v=6TH7kh3pS0E)


## How to use

1. Apply patch to the kernel from kmod/kernel-patch and then rebuild the kernel

Upatch is based on the UPROBE mechanism. We make some modifications for the kernel.

We have send these patch to the kernel, but it sitll not be accepted.

Patch list:
+ [UPROBE_ALTER_PC](https://www.spinics.net/lists/kernel/msg4516532.html)

2. build the project and install it
```
mkdir build
cd build
cmake ..
make && make install
cd -
```

3. prepare parameters for the compiler(default is gcc from SYMTEM env)
```
echo "/usr/bin/gcc" > /sys/kernel/upatch/compiler_path
echo "/usr/bin/as" > /sys/kernel/upatch/assembler_path
```


## Limitaion


## TODO list
1. previliage for sysfs (allow no-root)
2. support mutiple compilers work at the same time
3. gcc don't support -g3 (.debug_macro is group section)













