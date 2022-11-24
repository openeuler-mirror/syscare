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
2.  cd syscare
3.  mkdir tmp
4.  cd tmp
5.  cmake ..
6.  make
7.  make install 
```
rpm安装：
1.rpm -ivh syscare-<version>.rpm

二进制安装：
1. 正确配置dfn/yum仓库文件.
2. dnf update & dnf install syscare.
3. enjoy the tool.

#### 使用说明

补丁制作
```
syscare-build --name redis_cve_2021_32675 \
        --source redis-6.2.5-1.src.rpm \
        --debuginfo redis-debuginfo-6.2.5-1.x86_64.rpm \
        --target-elfname redis-server \
        --summary CVE-2021-32675 \
        0001-Prevent-unauthenticated-client-from-easily-consuming.patch
```
补丁制作详细参数见syscare/build/README.md

补丁管理
```
1. 补丁安装
syscare apply redis_cve_2021_32675

2. 补丁激活：
syscare active redis_cve_2021_32675

3. 补丁去激活：
syscarae deactive redis_cve_2021_32675

4. 补丁卸载/移除：
syscare remove redis_cve_2021_32675
补丁只有在deactive的状态才能移除

5. 补丁状态查询：
syscare status redis_cve_2021_32675

6. 查询syscare所有补丁：
syscare patch list

```

#### 示例

补丁制作
内核补丁制作：
```
syscare-build --name redis_cve_2021_32675 \
        --source redis-6.2.5-1.src.rpm \
        --debuginfo redis-debuginfo-6.2.5-1.x86_64.rpm \
        --target-elfname redis-server \
        --summary CVE-2021-32675 \
        0001-Prevent-unauthenticated-client-from-easily-consuming.patch
```

补丁管理

1. 补丁安装
syscare apply test

2. 补丁激活：
syscare active test

3. 补丁去激活：
syscarae deactive test

4. 补丁卸载/移除：
syscare remove test
补丁只有在deactive的状态才能移除

5. 补丁状态查询：
syscare status test

6. 查询syscare所有补丁：
syscare patch list

#### 约束限制
1. 版本约束:

内核版本：本期syscare仅支持openEuler22.03 LTS sp1

2. 应用约束

用户态补丁当前仅支持：redis、nginx、mysql
ps:当前对LINE宏的处理需要对每个软件进行适配，当前仅考虑适配redis、nginx、mysql，其他未适配的软件可能会造成patch的size过大(后续会考虑引入参数支持用户自行适配)

3. 语言约束

原理上补丁制作在object file一级进行比较，与编程语言无关，当前仅测试了c语言

4. 其他约束

* 暂不支持交叉编译
* 补丁管理操作需要root权限
* 使用的debug信息格式必须为dwarf，且不支持g3等级的调式信息

#### 参与贡献

1.  Fork 本仓库 $ git clone https://gitee.com/openeuler/syscare.git
2.  建立自己分支 $ cd syscare & git branch -b Feature_XXXX
3.  完善特性代码 $ vim src/upatch/xxxx  & git commit -m ""
4.  提交代码 $ git push origin
5.  新建 Pull Request


