# SysCare: 系统热服务

## 介绍

​       SysCare是一个系统级热修复软件，为操作系统提供单机级与集群级安全补丁和系统错误热修复，主机无需重新启动即可修复该系统问题。
​       当前仅融合统一内核态/用户态热补丁技术，用户需聚焦在自己核心业务中，系统修复问题交予SysCare进行处理。后期计划根据修复组件的不同，提供系统热升级技术，进一步解放运维用户提升运维效率。



## 软件架构

​       可以利用系统组件源代码与相应的patch问题，制作出相应组件补丁的RPM（包含补丁文件、依赖信息与配置信息等）. 制作的补丁RPM，可以上传到相应的补丁仓库中，集群的系统demon定时去查询补丁仓库, 对系统中运行的CVE与软件错误进行热修复，保证系统安全、稳定、高效运行。



## 安装教程

### 依赖安装

```bash
$ dnf install -y kernel-source-`uname -r` kernel-debuginfo-`uname -r` kernel-devel-`uname -r`
$ dnf install -y elfutils-libelf-devel openssl-devel dwarves python3-devel rpm-build bison cmake make gcc g++
```

### 源代码编译安装

```bash
git clone https://gitee.com/openeuler/syscare.git
cd syscare
mkdir tmp
cd tmp
cmake ..
make
make install
```

### rpm安装

1. ```rpm -ivh syscare-<version>.rpm```

### 二进制安装

1. 正确配置dnf/yum仓库文件
2. ```dnf update & dnf install syscare```
3. enjoy the tool



## 使用说明



### 补丁制作

```
$ syscare build \
   --patch-name "HP001" \
   --patch-description "CVE-2021-32675" \
   --source ./redis-6.2.5-1.src.rpm \
   --debuginfo ./redis-debuginfo-6.2.5-1.x86_64.rpm \
   --output ./output \
   ./0001-Prevent-unauthenticated-client-from-easily-consuming.patch
```

补丁制作详细使用说明请见build/README.md

### 补丁管理

1. 补丁安装

```bash
$ syscare apply redis-6.2.5-1/HP001
```

2. 补丁激活

```bash
$ syscare active redis-6.2.5-1/HP001
```

3. 补丁去激活

```bash
$ syscarae deactive redis-6.2.5-1/HP001
```

4. 补丁卸载/移除

```bash
$ syscare remove redis-6.2.5-1/HP001
```

5. 查询补丁状态

```bash
$ syscare status redis-6.2.5-1/HP001
```

6. 查询补丁信息

```bash
$ syscare info redis-6.2.5-1/HP001
```

7. 查询补丁目标软件信息

```bash
$ syscare target redis-6.2.5-1/HP001
```

8. 查询所有补丁

```bash
$ syscare list
```

### 系统管理

1. 快速重启系统

```bash
$ syscare reboot
```

命令行 详细使用说明请见cli/README.md



## 约束限制

* 当前支持ELF格式的热修复，解释型语言不支持；
* 支持debug信息格式为DWARF，且不支持g3等级的调试信息；
* 当前暂不支持交叉编译；



## 参与贡献

1.  Fork 本仓库 ```$ git clone https://gitee.com/openeuler/syscare.git```
2.  建立自己分支 ```$ cd syscare & git branch -b Feature_XXXX```
3.  完善特性代码 ```$ vim src/upatch/xxxx  & git commit -m ""```
4.  提交代码 ```$ git push origin```
5.  新建 Pull Request



## 参与讨论

* 可添加微信号: syscare, 申请加入syscare讨论群
* 可在openEuler论坛发帖或相应帖子中回复: [https://forum.openeuler.org/](https://forum.openeuler.org/)
