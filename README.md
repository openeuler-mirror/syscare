# SysCare: 系统热服务

#### 介绍
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;SysCare 是一个系统级热修复软件，为操作系统提供单机级与集群级安全补丁和系统错误热修复，主机无需重新启动即可修复该系统问题。<br />
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;当前仅融合统一内核态/用户态热补丁技术，用户需聚焦在自己核心业务中，系统修复问题交予SysCare进行处理。后期计划根据修复组件的不同，提供系统热升级技术，进一步解放运维用户提升运维效率。

#### 软件架构
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;SysCare 可以利用系统组件源代码与相应的patch问题，制作出相应组件补丁的RPM（包含补丁文件、依赖信息与配置信息等）. 制作的补丁RPM，可以上传到相应的补丁仓库中，集群的系统demond定时去查询补丁仓库, 对系统中运行的CVE与软件错误进行热修复，保证系统安全、稳定、高效运行。


#### 安装教程
依赖安装：
```
1.yum install -y kernel-source-`uname -r` kernel-debuginfo-`uname -r` kernel-devel-`uname -r` 
2.yum install -y elfutils-libelf-devel openssl-devel dwarves python3-devel rpm-build bison cmake make gcc g++
```

源代码编译安装：
```
1.  git clone https://gitee.com/openeuler/syscare.git
2.  cd syscare & make rpm
3.  rpm -ivh syscare-<version>.rpm
```

二进制安装：
1. 正确配置dfn/yum仓库文件；
2. dnf update & dnf install syscare
3. enjoy the tool.

#### 使用说明

```
1.  syscare build --help	具体参数见build/README.md
2.  syscare apply patch-name
3.  syscare active patch-name
4.  syscare deactive patch-name
5.  syscare remove patch-name
```

#### 示例

####源码编译


####补丁制作
内核补丁制作：
	syscare build --patch-name test --source ./kernel-xxxx.oexx.src.rpm --debug-info ./vmlinux ./test.patch
用户态补丁制作：
	syscare build --patch-name redis-test --source ./redis-xxx.rpm --target redis-server --debug-info ./redis ./redis-test.patch

####补丁管理
```
1.补丁安装
syscare apply test

2.补丁激活：
syscare active test

3.补丁去激活：
syscarae deactive test

4.补丁卸载/移除：
syscare remove test
补丁只有在deactive的状态才能移除

5.补丁状态查询：
syscare status test

6.查询syscare所有补丁：
syscare patch list

```

#### 参与贡献

1.  Fork 本仓库 $ git clone https://gitee.com/openeuler/syscare.git
2.  建立自己分支 $ cd syscare & git branch -b Feature_XXXX
3.  完善特性代码 $ vim src/upatch/xxxx  & git commit -m ""
4.  提交代码 $ git push origin
5.  新建 Pull Request


