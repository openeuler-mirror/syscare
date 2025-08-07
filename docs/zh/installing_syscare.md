# 安装SysCare

本章介绍在openEuler中安装SysCare的方法。

## 安装SysCare核心组件

### 最低硬件要求

* 2 CPU (x86_64 / aarch64) 
* 4GB RAM
* 100GB 硬盘

### 前提条件

安装openEuler 25.03版本。

### 源码编译安装SysCare

SysCare源码已经归档至代码仓<https://gitee.com/openeuler/syscare.git>，用户可自行下载并编译安装。

SysCare在编译前需要安装依赖包，相关命令如下：

```shell
dnf install cmake make rust cargo kernel-devel elfutils-libelf-devel llvm clang bpftool libbpf libbpf-devel libbpf-static
```

示例如下：

```shell
git clone https://gitee.com/openeuler/syscare.git
cd syscare
mkdir build
cd build
cmake -DCMAKE_INSTALL_PREFIX=/usr -DKERNEL_VERSION=$(uname -r) ..
make
make install
```

### repo安装SysCare

如果repo源中有SysCare相关的包，则可以通过dnf或yum命令进行下载、安装。

相关命令如下：

```shell
dnf install syscare syscare-kmod syscare-build syscare-build-ebpf
```

### 卸载SysCare

相关命令如下：

```shell
dnf erase syscare syscare-kmod syscare-build syscare-build-ebpf
```
