# SysCare: System hot service

## Introduction

SysCare is a system-level hot repair software that provides stand-alone and  cluster-level security patches and system error hot fixes for the  operating system. The host can repair system problems without  restarting. Currently, only unified kernel mode/user mode hot patch  technology is integrated. Users need to focus on their core business,  and system repair issues are handled by SysCare. In the later stage, we  plan to provide system hot upgrade technology based on different  repaired components to further liberate operation and maintenance users  and improve operation and maintenance efficiency.

## Software Architecture

You can use the system component source code and the corresponding  patch issues to produce the RPM of the corresponding component patch  (including patch files, dependency information, configuration  information, etc.). The produced patch RPM can be uploaded to the  corresponding patch warehouse, cluster system Demon regularly queries  the patch warehouse and hot-fixes CVEs and software errors running in  the system to ensure safe, stable and efficient operation of the system.

## Installation tutorial

### DNF installation

1. Correctly configure the dnf/yum warehouse file
2. `dnf update & dnf install syscare`
3. enjoy the tool

### Source code compilation and installation

- Install compilation dependencies

  ```
  $ kernel-version=$(uname -r)
  $ dnf install -y kernel-source-$kernel-version kernel-debuginfo-$kernel-version kernel-devel-$kernel-version
  $ dnf install -y elfutils-libelf-devel openssl-devel dwarves python3-devel rpm-build bison cmake make gcc g++
  ```

- Compile and install

  ```
  git clone https://gitee.com/openeuler/syscare.git
  cd syscare
  mkdir tmp
  cd tmp
  cmake -DCMAKE_INSTALL_PREFIX=/usr -DKERNEL_VERSION=$(uname -r) ..
  make
  make install
  ```

### rpm installation

```
rpm -ivh syscare-*.rpm
```

## Instructions for use

### patch making

```
$ syscare build \
   --patch-name "HP001" \
   --patch-description "CVE-2021-32675" \
   --source ./redis-6.2.5-1.src.rpm \
   --debuginfo ./redis-debuginfo-6.2.5-1.x86_64.rpm \
   --output ./output \
   --patch ./0001-Prevent-unauthenticated-client-from-easily-consuming.patch
```

### Kernel module hot patch production

```
$ syscare build \
   --patch-name HP001 \
   --source ./kernel-5.10.0-60.91.0.115.src.rpm \
   --source ./testmod-1-1.src.rpm \
   --debuginfo ./kernel-debuginfo-5.10.0-60.91.0.115.aarch64.rpm \
   --output ./output \
   --verbose \
   --skip-cleanup \
   --patch ./0001-test.patch
```

For detailed instructions on patch production, please see syscare-build/README.md

### Patch management

1. Patch installation

```
$ sudo syscare apply redis-6.2.5-1/HP001
```

1. patch activation

```
$ sudo syscare active redis-6.2.5-1/HP001
```

1. Patch deactivation

```
$ sudo syscarae deactive redis-6.2.5-1/HP001
```

1. Patch uninstall/removal

```
$ sudo syscare remove redis-6.2.5-1/HP001
```

1. Confirm patch

```
$ sudo syscare accept redis-6.2.5-1/HP001
```

1. Query patch status

```
$ syscare status redis-6.2.5-1/HP001
```

1. Query patch information

```
$ syscare info redis-6.2.5-1/HP001
```

1. Query patch target software information

```
$ syscare target redis-6.2.5-1/HP001
```

1. Query all patches

```
$ syscare list
```

### System Management

1. Quickly restart the system

```
$ syscare reboot
```

For detailed command line instructions, please see cli/README.md

## Constraints

- Currently only supports 64-bit systems;
- Currently, only ELF format hot fixes are supported, and interpreted languages ​​are not supported yet;
- Currently only gcc/g++ compilers are supported;
- The compiler needs to support `-gdwarf` `-ffunction-sections` `-fdata-sections` parameters;
- Only supports debugging information in DWARF format, and does not support g3 level;
- Modification of global variables is not supported;
- Cross-compilation is not supported yet;
- Assembly modification is not supported yet;
- New external symbols (dynamic library dependencies) are not supported yet;
- Multiple patches to the same binary are not currently supported;
- Patch file names with the same name are not supported for the time being. Bind is Local and Type is `STT_FUNC` or `STT_OBJECT` . Exactly the same symbol modification: A file with the same name  exists, and the local variable and function names are exactly the same.  This is possible. inconsistent;
- C & C++ mixed compilation is not supported yet;
- C++ exception modification is not supported yet;
- Group section: `-g3` compilation options, specific compilation optimization options, specific gcc plugin, etc. are not supported yet;
- New ifunc: `__attribute__((ifunc("foo")))` is not supported yet;
- New TLS variables are not supported yet: `__thread int foo` ;
- Compiling with the LTO option is not currently supported.

## Participate and contribute

1. Fork this repository `$ git clone https://gitee.com/openeuler/syscare.git` 
2. Create your own branch `$ cd syscare & git branch -b Feature_XXXX` 
3. Improve feature code `$ vim src/upatch/xxxx & git commit -m ""` 
4. Submit code `$ git push origin` 
5. New Pull Request

## Participate in discussions

- You can add WeChat ID: syscare, apply to join the syscare discussion group
- You can post in the openEuler forum or reply in the corresponding thread: https://forum.openeuler.org/