# Installing SysCare

This chapter describes how to install SysCare on openEuler.

## Installing SysCare Core Components

### Minimum Hardware Requirements

* 2 CPUs (x86_64 or AArch64)
* 4 GB memory
* 100 GB drive

### Prerequisites

1. openEuler 24.03 LTS SP1 has been installed.

### Installing from Source

Clone the SysCare source code <https://gitee.com/openeuler/syscare.git> and then compile and install SysCare.

Before compilation, install dependencies:

```shell
dnf install cmake make rust cargo kernel-devel elfutils-libelf-devel llvm clang bpftool libbpf libbpf-devel libbpf-static
```

Compile and install SysCare:

```shell
git clone https://gitee.com/openeuler/syscare.git
cd syscare
mkdir build
cd build
cmake -DCMAKE_INSTALL_PREFIX=/usr -DKERNEL_VERSION=$(uname -r) ..
make
make install
```

### Installing SysCare from a Repository

If the repository source contains SysCare packages, you can use the `dnf` or `yum` command to download and install them.

```shell
dnf/yum install syscare syscare-kmod syscare-build syscare-build-ebpf
```

### Uninstalling SysCare

```shell
dnf/yum erase syscare syscare-kmod syscare-build syscare-build-ebpf
```
