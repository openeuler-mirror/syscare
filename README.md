# SysCare: 系统热服务

## 介绍

​		SysCare是一个系统级热修复软件，为操作系统提供单机级与集群级安全补丁和系统错误热修复，主机无需重新启动即可修复该系统问题。
​		当前仅融合统一内核态/用户态热补丁技术，用户需聚焦在自己核心业务中，系统修复问题交予SysCare进行处理。后期计划根据修复组件的不同，提供系统热升级技术，进一步解放运维用户提升运维效率。



## 软件架构

​		可以利用系统组件源代码与相应的patch问题，制作出相应组件补丁的RPM（包含补丁文件、依赖信息与配置信息等）. 制作的补丁RPM，可以上传到相应的补丁仓库中，集群的系统daemon定时去查询补丁仓库, 对系统中运行的CVE与软件错误进行热修复，保证系统安全、稳定、高效运行。



## 安装教程

### DNF安装

1. 正确配置dnf/yum仓库文件
2. ```dnf update & dnf install syscare```
3. enjoy the tool

### 源码编译安装

  * 安装编译依赖

    ```bash
    $ kernel-version=$(uname -r)
    $ dnf install -y kernel-source-$kernel-version kernel-debuginfo-$kernel-version kernel-devel-$kernel-version
    $ dnf install -y elfutils-libelf-devel openssl-devel dwarves python3-devel rpm-build bison cmake make gcc g++
    ```

  * 编译并安装

    PS: 直接编译在应用补丁的时候会显示缺少依赖，建议通过rpm包安装

    ```bash
    git clone https://gitee.com/openeuler/syscare.git
    cd syscare
    mkdir tmp
    cd tmp
    cmake -DCMAKE_INSTALL_PREFIX=/usr -DKERNEL_VERSION=$(uname -r) ..
    make
    make install

    mkdir -p /usr/lib/syscare/patches
    systemctl daemon-reload
    systemctl enable syscare
    systemctl start syscare
    ```

  * 离线编译
    首先在网络的环境上执行cargo vendor下载所有依赖到./vendor目录下
    ```
    cd syscare
    cargo vendor
    ```
    源码目录创建.cargo/config.toml，并写入以下设置，下次编译就不需要联网了
    ```
    [source.crates-io]
    replace-with = "vendored-sources"

    [source.vendored-sources]
    directory = "vendor"
    ```

### rpm安装

```bash
rpm -ivh syscare-*.rpm
```
或者

```
dnf install syscare-*.rpm
```


## 使用说明

### 补丁制作

```
$ syscare build \
   --patch-name "HP001" \
   --patch-description "CVE-2021-32675" \
   --source ./redis-6.2.5-1.src.rpm \
   --debuginfo ./redis-debuginfo-6.2.5-1.x86_64.rpm \
   --output ./output \
   --patch ./0001-Prevent-unauthenticated-client-from-easily-consuming.patch
```

### 内核模块热补丁制作
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

补丁制作详细使用说明请见[syscare-build/README.md](https://gitee.com/openeuler/syscare/blob/master/syscare-build/README.md)



### 补丁管理

1. 补丁安装

```bash
$ sudo syscare apply redis-6.2.5-1/HP001
```

2. 补丁激活

```bash
$ sudo syscare active redis-6.2.5-1/HP001
```

3. 补丁去激活
```bash
$ sudo syscare deactive redis-6.2.5-1/HP001
```

4. 补丁卸载/移除

```bash
$ sudo syscare remove redis-6.2.5-1/HP001
```

5. 确认补丁

```bash
$ sudo syscare accept redis-6.2.5-1/HP001
```

6. 查询补丁状态

```bash
$ syscare status redis-6.2.5-1/HP001
```

7. 查询补丁信息

```bash
$ syscare info redis-6.2.5-1/HP001
```

8. 查询补丁目标软件信息

```bash
$ syscare target redis-6.2.5-1/HP001
```

9. 查询所有补丁

```bash
$ syscare list
```



### 系统管理

1. 快速重启系统

```bash
$ syscare reboot
```

命令行详细使用说明请见[cli/README.md](https://gitee.com/openeuler/syscare/blob/master/cli/README.md)



## 约束限制

* 当前仅支持64位系统；
* 当前仅支持ELF格式的热修复，暂不支持解释型语言；
* 当前仅支持gcc / g++编译器；
* 编译器需要支持```-gdwarf``` ```-ffunction-sections``` ```-fdata-sections```参数；
* 仅支持DWARF格式的调试信息，且不支持g3等级；
* 不支持修改全局变量；
* 暂不支持交叉编译；
* 暂不支持汇编修改；
* 暂不支持新增外部符号（动态库依赖）；
* 暂不支持对同一个二进制打多个补丁；
* 暂不支持补丁文件名相同，Bind为Local并且Type为```STT_FUNC```或```STT_OBJECT```完全相同的符号修改：
  存在同名文件，并且局部变量和函数名称完全一致，实现可能不一致；
* 暂不支持C & C++ 混合编译；
* 暂不支持C++ exception修改；
* 暂不支持group section: ```-g3```编译选项，特定编译优化选项，特定gcc plugin等；
* 暂不支持新增ifunc: ```__attribute__((ifunc("foo")))```；
* 暂不支持新增TLS变量: ```__thread int foo```；
* 暂不支持编译开启LTO选项。



## 参与贡献

1.  Fork 本仓库 ```$ git clone https://gitee.com/openeuler/syscare.git```
2.  建立自己分支 ```$ cd syscare & git branch -b Feature_XXXX```
3.  完善特性代码 ```$ vim src/upatch/xxxx & git commit -m ""```
4.  提交代码 ```$ git push origin```
5.  新建 Pull Request



## 参与讨论

* 可添加微信号: syscare, 申请加入syscare讨论群
* 可在openEuler论坛发帖或相应帖子中回复: [https://forum.openeuler.org/](https://forum.openeuler.org/)
