# 使用SysCare

本章介绍在openEuler中使用SysCare的方法。

## 前提条件

安装openEuler 24.03 LTS SP1版本。

## SysCare使用

本章节将介绍 SysCare 的使用方法，包含热补丁制作及热补丁管理。

### 热补丁制作

用户可以使用`sycare build`命令制作热补丁，该命令为纯CLI工具，提供从RPM包生成热补丁包的功能，热补丁包以RPM包的形式封装维护，支持制作内核热补丁及用户态热补丁。

#### 热补丁制作流程

1. 准备热补丁目标软件源码包(source rpm)及软件调试信息包(debuginfo rpm)

   示例：

   ```shell
   yumdownloader kernel --source --debuginfo
   ```

2. 确认满足对应软件编译依赖

   示例：

   ```shell
   dnf install make gcc bison flex openssl-devel dwarves python3-devel elfutils-libelf-devel
   ```

3. 执行`syscare build`命令构建热补丁

   示例：

   ```shell
   syscare build \
           --patch_name HP001 \
           --source kernel-5.10.0-60.66.0.91.oe2203.src.rpm \
           --debuginfo kernel-debuginfo-5.10.0-60.66.0.91.oe2203.x86_64.rpm \
           --output output \
           --patch 001-kernel-patch-test.patch
   ```

   热补丁制作过程将会在由`--workdir`参数所指定的目录中（默认为当前目录）创建以`syscare-build`开头的临时文件夹，用于存放临时文件及编译日志。

   示例：

   ```shell
   dev@openeuler-dev:[~]$ ls -l syscare-build.111602/
   total 100
   -rw-r--r--. 1 dev dev 92303 Nov 12 00:00 build.log
   drwxr-xr-x. 6 dev dev 4096 Nov 12 00:00 package
   drwxr-xr-x. 4 dev dev 4096 Nov 12 00:00 patch
   ```

   编译日志将会生成在临时文件夹中，名称为`build.log`。

   ```shell
   dev@openeuler-dev:[~]$ cat syscare-build.111602/build.log | less
   ```

   若补丁制作成功，且未指定`--skip-compiler-check`参数，将自动删除该临时文件夹。

4. 检查编译结果

   示例：

   ```shell
   dev@openeuler-dev:[~]$ ls -l
   total 189680
   -rw-r--r--. 1 dev dev 194218767 Nov 12 00:00 kernel-5.10.0-60.91.0.115.oe2203-HP001-1-1.x86_64.src.rpm
   -rw-r--r--. 1 dev dev     10937 Nov 12 00:00 patch-kernel-5.10.0-60.91.0.115.oe2203-HP001-1-1.x86_64.rpm
   ```

   其中

   - `patch-kernel-5.10.0-60.91.0.115.oe2203-HP001-1-1.x86_64.rpm`为补丁包
   - `kernel-5.10.0-60.91.0.115.oe2203-HP001-1-1.x86_64.src.rpm`为二进制包

#### 热补丁制作工具

```shell
USAGE:
    syscare build [OPTIONS] --patch_name <PATCH_NAME> --source <SOURCE> --debuginfo <DEBUGINFO>... --patch <PATCH_FILES>...

OPTIONS:
    -n, --patch_name <PATCH_NAME>                  Patch name
        --patch_arch <PATCH_ARCH>                  Patch architecture [default: x86_64]
        --patch_version <PATCH_VERSION>            Patch version [default: 1]
        --patch_release <PATCH_RELEASE>            Patch release [default: 1]
        --patch_description <PATCH_DESCRIPTION>    Patch description [default: (none)]
    -s, --source <SOURCE>                          Source package
    -d, --debuginfo <DEBUGINFO>...                 Debuginfo package(s)
        --workdir <WORKDIR>                        Working directory [default: .]
    -o, --output <OUTPUT>                          Output directory [default: .]
    -j, --jobs <JOBS>                              Parllel build jobs [default: 96]
        --skip_compiler_check                      Skip compiler version check (not recommended)
        --skip_cleanup                             Skip post-build cleanup
    -v, --verbose                                  Provide more detailed info
    -p, --patch <PATCH_FILES>...                   Patch file(s)
    -h, --help                                     Prints help information
    -V, --version                                  Prints version information
```

|名称|描述|类型|备注|
| ---- | ---- | ---- | ---- |
|-n, --patch_name `<PATCH_NAME>`|补丁名称|字符串|必选参数，需符合RPM命名规范|
|--patch_arch `<PATCH_ARCH>`|补丁架构|字符串|默认为当前架构，需符合RPM命名规范|
|--patch_version `<PATCH_VERSION>`|补丁版本号|字符串|默认值为1，需符合RPM命名规范|
|--patch_release `<PATCH_RELEASE>`|补丁release|数字|默认值为1，需符合RPM命名规范|
|--patch_description `<PATCH_DESCRIPTION>`|补丁描述|字符串|默认为(none)|
|-s, --source `<SOURCE>`|目标软件src.rpm源码包路径|字符串|必选参数，需为合法路径|
|-d, --debuginfo `<DEBUGINFO>...`|目标软件debuginfo包路径|字符串|必选参数，可指定多个，需为合法路径|
|--workdir `<WORKDIR>`|临时文件夹路径|字符串|默认为当前执行目录，需为合法路径|
|-o, --output `<OUTPUT>`|补丁输出文件夹|字符串|默认为当前执行目录，需为合法路径|
|-j, --jobs `<JOBS>`|并行编译线程数|数字|默认为cpu线程数|
|--skip-compiler-check|跳过编译器检查|标识|-|
|--skip-cleanup|跳过临时文件清理|标识|-|
|-v, --verbose|打印详细信息|标识|-|
|-p, --patch `<PATCHES>...`|补丁文件路径|字符串|必选参数，可指定多个，需为合法路径|
|-h, --help|打印帮助信息|标识|-|
|-V, --version|打印版本信息|标识|-|

示例：

```shell
syscare build \
    --patch_name "HP001" \
    --patch_description "CVE-2021-32675 - When parsing an incoming Redis Standard Protocol (RESP) request, Redis allocates memory according to user-specified values which determine the number of elements (in the multi-bulk header) and size of each element (in the bulk header). An attacker delivering specially crafted requests over multiple connections can cause the server to allocate significant amount of memory. Because the same parsing mechanism is used to handle authentication requests, this vulnerability can also be exploited by unauthenticated users." \
    --source ./redis-6.2.5-1.src.rpm \
    --debuginfo ./redis-debuginfo-6.2.5-1.x86_64.rpm \
    --output ./output \
    --patch ./0001-Prevent-unauthenticated-client-from-easily-consuming.patch
```

#### 热补丁包命名规则

- 热补丁包：patch-目标软件全名-补丁名称-补丁版本-补丁release.架构名.rpm
- 热补丁源码包：目标软件全名-补丁名称-补丁版本-补丁release.架构名.src.rpm

#### 输出

- 热补丁包：包含SysCare热补丁的二进制及元信息，用于热补丁安装。
- 热补丁源码包：包含目标软件源码及新增补丁文件，用于新版本热补丁制作。

#### 错误定位

如果热补丁制作过程出现错误，请参考位于工作目录下名称为`build.log`的编译日志。

示例：

```shell
Building patch, this may take a while
- Preparing build requirements
- Building patch
Error: UserPatchBuilder: Failed to build patch

Caused by:
    Process "/usr/libexec/syscare/upatch-build" exited unsuccessfully, exit_code=253
For more information, please check "/home/dev/syscare-build.345549/build.log"
```

### 热补丁包管理

热补丁的安装以及卸载需要提供对应rpm包的名称，下面使用`$patch_package`来指代rpm包名称。

1. 热补丁包安装

   ```shell
   dnf install $patch_package.rpm
   ```

   热补丁包安装后，热补丁相关文件存放在如下路径：

   /usr/lib/syscare/patches

2. 热补丁包卸载

   ```shell
   dnf remove $patch_package
   ```

   注：若热补丁处于`ACTIVED`以上状态时，热补丁将会被自动卸载。

### 热补丁管理

使用`syscare`命令可以对热补丁进行管理。

对单一热补丁操作前，用户需要提供一个字符串来搜索热补丁，后续使用`$patch_identifier`来指代这个字符串。

热补丁管理搜索规则为：目标包名/补丁名，其中“目标包名/”在补丁名唯一的情况下可以省略，也可使用UUID来进行管理。

*目标包名：待打入补丁的目标软件的软件包名称；
*补丁名：热补丁名称。

#### 补丁元信息

补丁元信息中包含以下字段：

| 字段名称 | 字段描述 |
| ----------- | ---------------------- |
| uuid | 补丁ID |
| name | 补丁名称 |
| version | 补丁版本 |
| release | 补丁Release |
| arch | 补丁架构 |
| type | 补丁类型 |
| target | 目标软件名 |
| entities | 目标软件可执行文件名称 |
| digest | 补丁指纹 |
| license | 目标软件许可证 |
| description | 补丁描述 |
| patch| 补丁文件列表 |

示例：

```shell
sudo syscare info redis-6.2.5-1/HP002-1-1
uuid:        980fa0d0-e753-447c-8494-01de595f35d0
name:        HP002
version:     1
release:     1
arch:        x86_64
type:        UserPatch
target:      redis-6.2.5-1
target_elf:  redis-server, redis-benchmark, redis-cli
license:     BSD and MIT
description: CVE-2021-32675 - When parsing an incoming Redis Standard Protocol (RESP) request, Redis allocates memory according to user-specified values which determine the number of elements (in the multi-bulk header) and size of each element (in the bulk header). An attacker delivering specially crafted requests over multiple connections can cause the server to allocate significant amount of memory. Because the same parsing mechanism is used to handle authentication requests, this vulnerability can also be exploited by unauthenticated users.
patch:
0001-Prevent-unauthenticated-client-from-easily-consuming.patch
```

#### 热补丁状态

SysCare将热补丁的生命周期分成如下状态：

*未加载：`NOT-APPLIED`
*未激活：`DEACTIVED`
*已激活：`ACTIVED`
*已接受：`ACCEPTED`

#### 补丁信息查询

1. 补丁基本信息查询：

   ```shell
   syscare info $patch_identifier
   ```

2. 补丁状态查询：

   ```shell
   syscare status $patch_identifier
   ```

3. 查询所有补丁状态：

   ```shell
   syscare list
   ```

#### 热补丁状态管理

1. 加载热补丁：

   ```shell
   syscare apply $patch_identifier
   ```

2. 卸载热补丁：

   ```shell
   syscare remove $patch_identifier
   ```

3. 激活热补丁：

   ```shell
   syscare active $patch_identifier
   ```

4. 反激活热补丁：

   ```shell
   syscare deactive $patch_identifier
   ```

5. 接受热补丁：

   ```shell
   syscare accept $patch_identifier
   ```

6. 保存所有补丁状态：

   ```shell
   syscare save
   ```

7. 恢复所有补丁状态：

   ```shell
   syscare restore
   ```
