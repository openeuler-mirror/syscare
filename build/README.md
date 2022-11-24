# Syscare build

Syscare 补丁制作工具

syscare build为纯CLI工具，提供从RPM包生成热补丁包的功能，补丁包以RPM包的形式封装维护，支持制作内核热补及用户态热补丁。



## 命令行参数

```
Usage: syscare build [OPTIONS] --name <NAME> --source <SOURCE> --debuginfo <DEBUGINFO> <PATCHES>...

Arguments:
  <PATCHES>...  Patch file(s)

Options:
  -n, --name <NAME>                      Patch name
      --version <VERSION>                Patch version [default: 1]
      --summary <SUMMARY>                Patch summary [default: "syscare patch"]
      --target-name <TARGET_NAME>        Patch target name
  -t, --target-elfname <TARGET_ELFNAME>  Patch target executable name
      --target-version <TARGET_VERSION>  Patch target version
      --target-release <TARGET_RELEASE>  Patch target release
      --target-license <TARGET_LICENSE>  Patch target license
  -s, --source <SOURCE>                  source package
  -d, --debuginfo <DEBUGINFO>            Debuginfo package
      --workdir <WORKDIR>                Working directory [default: .]
  -o, --output <OUTPUT>                  Generated patch output directory [default: .]
      --kjobs <N>                        Kernel make jobs [default: 32]
      --skip-compiler-check              Skip compiler version check (not recommended)
  -h, --help                             Print help information
```



必要参数：

| 字段名称         | 字段描述                               |
| ---------------- | -------------------------------------- |
| --name           | 补丁名称                               |
| --source         | 目标软件源码包                         |
| --debuginfo      | 目标软件调试信息包                     |
| --target-elfname | 目标软件可执行文件名（内核补丁可忽略） |
| <PATCHES>        | 补丁列表                               |



示例：

```
syscare-build --name redis_cve_2021_32675 \
	--source redis-6.2.5-1.src.rpm \
	--debuginfo redis-debuginfo-6.2.5-1.x86_64.rpm \
	--target-elfname redis-server \
	--summary CVE-2021-32675 \
	0001-Prevent-unauthenticated-client-from-easily-consuming.patch
```



## 输出

* 补丁包：包含syscare热补丁的二进制及元信息，用于热补丁安装。
* 补丁源码包：包含目标软件源码及新增补丁，用于新版本热补丁制作。



命名规则：

* 补丁包：patch-目标软件全名-补丁名称-补丁版本-补丁release.架构名.rpm

* 补丁源码包：目标软件全名.patched.补丁名称.补丁版本.补丁release.src.rpm

  

## 补丁信息

补丁元信息中包含以下字段：

| 字段名称 | 字段描述               |
| -------- | ---------------------- |
| name     | 补丁名称               |
| type     | 补丁类型               |
| target   | 目标软件名             |
| elf_name | 目标软件可执行文件名称 |
| license  | 目标软件许可证         |
| version  | 补丁版本               |
| release  | 补丁Release            |
| summary  | 补丁概要               |



示例：

```
Collecting patch info
------------------------------
name:     redis
type:     UserPatch
target:   redis-6.2.5-1
elf_name: redis-server
license:  BSD and MIT
version:  1
release:  31fc7544
summary:  CVE-2021-32675

patch list:
0001-CVE-2021-32675.patch 31fc7544
```



补丁安装位置：

`/usr/lib/syscare/patches/补丁名`



## 补丁制作流程

1. 准备补丁目标软件源码包(source rpm)及软件调试信息包(debuginfo rpm)

   示例：

   ```
   yumdownloader kernel --source
   yumdownloader kernel-debuginfo
   ```

2. 确认满足对应软件编译依赖

   示例：

   ```
   dnf install make gcc bison flex openssl-devel dwarves python3-devel elfutils-libelf-devel
   ```

3. 执行syscare build命令

   示例：

   ```
   syscare-build \
           --name kernel_version \
           --source kernel-5.10.0-60.66.0.91.oe2203.src.rpm \
           --debuginfo kernel-debuginfo-5.10.0-60.66.0.91.oe2203.x86_64.rpm \
           --output output \
           001-kernel-patch-test.patch
   ```

4. 检查编译结果

   示例：

   ```
   dev@openeuler-dev:[~]$ ls -l
   total 189680
   -rw-r--r--. 1 dev dev 194218767 Nov 12 00:00 kernel-5.10.0-60.66.0.91.oe2203.patched.kernel_version.1.c15c1a6a.src.rpm
   -rw-r--r--. 1 dev dev     10937 Nov 12 00:00 patch-kernel-5.10.0-60.66.0.91.oe2203-kernel_version-1-c15c1a6a.x86_64.rpm
   ```

   其中

   `patch-kernel-5.10.0-60.66.0.91.oe2203-kernel_version-1-c15c1a6a.x86_64.rpm`为补丁包

   `kernel-5.10.0-60.66.0.91.oe2203.patched.kernel_version.1.c15c1a6a.src.rpm`为二进制包



## 错误处理

如果出现错误，请参考编译日志：

   错误示例：

   ```
   ...
   Building patch, this may take a while
   ERROR: Process '/usr/libexec/syscare/upatch-build' exited unsuccessfully, exit_code=255
   ```

   编译日志将会生成在workdir下（由--workdir参数指定，默认为当前目录）以syscare-build开头的文件夹中，名称为build.log

   示例：

   ```
   dev@openeuler-dev:[~]$ ls -l syscare-build.111602/
   total 100
   -rw-r--r--. 1 dev dev 92303 Nov 12 00:01 build.log
   drwxr-xr-x. 6 dev dev  4096 Nov 12 00:00 package
   drwxr-xr-x. 4 dev dev  4096 Nov 12 00:00 patch
   ```

   ```
   dev@openeuler-dev:[~]$ cat syscare-build.111602/build.log | less
   ...
   ```



## 约束限制

1. 补丁名称中不能包含除`_`外的任意特殊符号；

2. 制作补丁前需要满足其源码包的编译依赖；

3. 补丁源码包版本需要与调试信息包完全一致；

4. 所有参数指定的文件及文件夹均需已存在；

5. 输入参数错误将不会保留任何日志；

6. 建议以非root用户权限运行本工具。

   
