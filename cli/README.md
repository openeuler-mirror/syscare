# Syscare build

Syscare 补丁制作工具

syscare build为纯CLI工具，提供从RPM包生成热补丁包的功能，补丁包以RPM包的形式封装维护，支持制作内核热补及用户态热补丁。



## 命令行参数

```bash
Usage: syscare build [OPTIONS] --patch-name <PATCH_NAME> --source <SOURCE> --debuginfo <DEBUGINFO> <PATCHES>...

Arguments:
  <PATCHES>...  Patch file(s)

Options:
  -n, --patch-name <PATCH_NAME>                Patch name
      --patch-arch <PATCH_ARCH>                Patch architecture [default: x86_64]
      --patch-version <PATCH_VERSION>          Patch version [default: 1]
      --patch-release <PATCH_RELEASE>          Patch release [default: 1]
      --patch-description <PATCH_DESCRIPTION>  Patch description [default: (none)]
      --target-name <TARGET_NAME>              Patch target name
      --target-arch <TARGET_ARCH>              parch target architecture
      --target-epoch <TARGET_EPOCH>            Patch target epoch
      --target-version <TARGET_VERSION>        Patch target version
      --target-release <TARGET_RELEASE>        Patch target release
      --target-license <TARGET_LICENSE>        Patch target license
  -s, --source <SOURCE>                        Source package
  -d, --debuginfo <DEBUGINFO>                  Debuginfo package
      --workdir <WORKDIR>                      Working directory [default: .]
  -o, --output <OUTPUT>                        Generated patch output directory [default: .]
      --jobs <N>                               Parallel build jobs [default: 96]
      --skip-compiler-check                    Skip compiler version check (not recommended)
      --skip-cleanup                           Skip post-build cleanup
  -v, --verbose                                Provide more detailed info
  -h, --help                                   Print help information
  -V, --version                                Print version information
```

### 参数
|名称|描述|类型|备注|
| ---- | ---- | ---- | ---- |
| ```<PATCHES>```... |补丁文件路径|字符串|必选参数，可指定多个，需为合法路径|

### 选项
|名称|描述|类型|备注|
| ---- | ---- | ---- | ---- |
|-n, --patch-name ```<PATCH_NAME>```|补丁名称|字符串|必选参数，需符合RPM命名规范|
|--patch-arch ```<PATCH_ARCH>```|补丁架构|字符串|默认为当前架构，需符合RPM命名规范|
|--patch-version ```<PATCH_VERSION>```|补丁版本号|字符串|默认值为1，需符合RPM命名规范|
|--patch-release ```<PATCH_RELEASE>```|补丁release|数字|默认值为1，需符合RPM命名规范|
|--patch-description ```<PATCH_DESCRIPTION>```|补丁描述|字符串|默认为(none)|
|--target-name ```<TARGET_NAME>```|目标软件rpm包名称|字符串|默认由'--source'参数提供的src.rpm包推导|
|--target-arch ```<TARGET_ARCH>```|目标软件rpm包架构|字符串|默认由'--source'参数提供的src.rpm包推导|
|--target-epoch ```<TARGET_EPOCH>```|目标软件rpm包epoch|字符串|默认由'--source'参数提供的src.rpm包推导|
|--target-version ```<TARGET_VERSION>```|目标软件rpm包版本号|字符串|默认由'--source'参数提供的src.rpm包推导|
|--target-release ```<TARGET_RELEASE>```|目标软件rpm包release|字符串|默认由'--source'参数提供的src.rpm包推导|
|--target-license ```<TARGET_LICENSE>```|目标软件rpm包license|字符串|默认由'--source'参数提供的src.rpm包推导|
|-s, --source ```<SOURCE>```|目标软件src.rpm源码包路径|字符串|必选参数，需为合法路径|
|-d, --debuginfo ```<DEBUGINFO>```|目标软件debuginfo包路径|字符串|必选参数，需为合法路径|
|--workdir ```<WORKDIR>```|临时文件夹路径|字符串|默认为当前执行目录，需为合法路径|
|-o, --output ```<OUTPUT>```|补丁输出文件夹|字符串|默认为当前执行目录，需为合法路径|
|-j, --jobs ```<N>```|并行编译线程数|数字|默认为cpu线程数|
|--skip-compiler-check|跳过编译器检查|标识|-|
|--skip-cleanup|跳过临时文件清理|标识|-|
|-v, --verbose|打印详细信息|标识|-|
|-h, --help|打印帮助信息|标识|-|
|-V, --version|打印版本信息|标识|-|

### 返回值
* 成功返回 0
* 错误返回255

### 输出

* 补丁包：包含syscare热补丁的二进制及元信息，用于热补丁安装。
* 补丁源码包：包含目标软件源码及新增补丁，用于新版本热补丁制作。

### 命名规则

* 补丁包：patch-目标软件全名-补丁名称-补丁版本-补丁release.架构名.rpm

* 补丁源码包：目标软件全名.补丁名称.补丁版本.补丁release.src.rpm

### 补丁包安装位置

```bash
/usr/lib/syscare/patches/${uuid}
```
### 示例

```bash
syscare build \
    --patch-name "HP001" \
    --patch-description "CVE-2021-32675 - When parsing an incoming Redis Standard Protocol (RESP) request, Redis allocates memory according to user-specified values which determine the number of elements (in the multi-bulk header) and size of each element (in the bulk header). An attacker delivering specially crafted requests over multiple connections can cause the server to allocate significant amount of memory. Because the same parsing mechanism is used to handle authentication requests, this vulnerability can also be exploited by unauthenticated users." \
    --source ./redis-6.2.5-1.src.rpm \
    --debuginfo ./redis-debuginfo-6.2.5-1.x86_64.rpm \
    --output ./output \
        ./0001-Prevent-unauthenticated-client-from-easily-consuming.patch
```



## 补丁信息

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
| target_elf | 目标软件可执行文件名称 |
| digest | 补丁指纹 |
| license | 目标软件许可证 |
| description | 补丁描述 |
| patch| 补丁文件列表 |


示例：

```bash
dev@openeuler-dev:[output]$ syscare info redis-6.2.5-1/HP001
uuid:        ec503257-aa75-4abc-9045-c4afdd7ae0f2
name:        HP001
version:     1
release:     1
arch:        x86_64
type:        UserPatch
target:      redis-6.2.5-1
target_elf:  redis-cli, redis-server, redis-benchmark
digest:      31fc7544
license:     BSD and MIT
description: CVE-2021-32675 - When parsing an incoming Redis Standard Protocol (RESP) request, Redis allocates memory according to user-specified values which determine the number of elements (in the multi-bulk header) and size of each element (in the bulk header). An attacker delivering specially crafted requests over multiple connections can cause the server to allocate significant amount of memory. Because the same parsing mechanism is used to handle authentication requests, this vulnerability can also be exploited by unauthenticated users.
patch:
31fc7544 0001-Prevent-unauthenticated-client-from-easily-consuming.patch
```




## 补丁制作流程

1. 准备补丁目标软件源码包(source rpm)及软件调试信息包(debuginfo rpm)

   示例：

   ```bash
   yumdownloader kernel --source
   yumdownloader kernel-debuginfo
   ```

2. 确认满足对应软件编译依赖

   示例：

   ```bash
   dnf install make gcc bison flex openssl-devel dwarves python3-devel elfutils-libelf-devel
   ```

3. 执行syscare build命令

   示例：

   ```bash
   syscare-build \
           --patch-name HP001 \
           --source kernel-5.10.0-60.66.0.91.oe2203.src.rpm \
           --debuginfo kernel-debuginfo-5.10.0-60.66.0.91.oe2203.x86_64.rpm \
           --output output \
           001-kernel-patch-test.patch
   ```

   补丁制作过程将会在由`--workdir`参数所指定的目录中（默认为当前目录）创建以```syscare-build```开头的临时文件夹，用于存放临时文件及编译日志。

   示例：

   ```bash
   dev@openeuler-dev:[kernel_patch]$ ls -l syscare-build.111602/
   total 100
   -rw-r--r--. 1 dev dev 92303 Nov 12 00:00 build.log
   drwxr-xr-x. 6 dev dev  4096 Nov 12 00:00 package
   drwxr-xr-x. 4 dev dev  4096 Nov 12 00:00 patch
   ```
   编译日志将会生成在临时文件夹中，名称为```build.log```
   ```bash
   dev@openeuler-dev:[kernel_patch]$ cat syscare-build.111602/build.log | less
   ...
   ```
   若补丁制作成功，将不会保留该临时文件夹。

4. 检查编译结果

   示例：

   ```bash
   dev@openeuler-dev:[output]$ ll
   total 372M
   -rw-r--r--. 1 dev dev 186M Nov 12 00:00 kernel-5.10.0-60.80.0.104.oe2203-HP001-1-1.x86_64.src.rpm
   -rw-r--r--. 1 dev dev  11K Nov 12 00:00 patch-kernel-5.10.0-60.80.0.104.oe2203-HP001-1-1.x86_64.rpm
   ```

   其中

   `kernel-5.10.0-60.80.0.104.oe2203-HP001-1-1.x86_64.src.rpm`为补丁源码包

   `patch-kernel-5.10.0-60.80.0.104.oe2203-HP001-1-1.x86_64.rpm`为补丁二进制包



## 错误处理

如果出现错误，请参考编译日志。

错误示例：

```bash
...
Building patch, this may take a while
ERROR: Process '/usr/libexec/syscare/upatch-build' exited unsuccessfully, exit_code=255
```



## 约束限制

1. 制作补丁需要满足其源码包编译依赖；

2. 补丁源码包版本需要与调试信息包完全一致；

3. 补丁包构建环境需要与debuginfo包构建环境保持完全一致；

4. 所有参数指定的文件及文件夹均需已存在；

5. 若输入参数错误，将不会保留任何日志；

6. 建议以非root用户权限运行本命令。
