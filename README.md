# SysCare: 系统热服务

#### 介绍
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;SysCare 是一个系统级热修复软件，为操作系统提供单机级与集群级安全补丁和系统错误热修复，主机无需重新启动即可修复该系统问题。<br />
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;当前仅融合统一内核态/用户态热补丁技术，用户需聚焦在自己核心业务中，系统修复问题交予SysCare进行处理。后期计划根据修复组件的不同，提供系统热升级技术，进一步解放运维用户提升运维效率。

#### 软件架构
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;SysCare 可以利用系统组件源代码与相应的patch问题，制作出相应组件补丁的RPM（包含补丁文件、依赖信息与配置信息等）. 制作的补丁RPM，可以上传到相应的补丁仓库中，集群的系统demond定时去查询补丁仓库, 对系统中运行的CVE与软件错误进行热修复，保证系统安全、稳定、高效运行。


#### 安装教程

源代码安装：
1.  git clone https://gitee.com/openeuler/syscare.git
2.  cd syscare & make rpm
3.  rpm -ivh syscare-<version>.rpm
4.  enjoy the tool.

二进制安装：
1. 正确配置dfn/yum仓库文件；
2. dnf update & dnf install syscare
3. enjoy the tool.

#### 使用说明

1.  syscare build <system package>.src.rpm xxxx.patch
2.  syscare apply <system pacakge>-patch.rpm
3.  syscare active <system package>
4.  syscare deactive <system packge>
5.  syscare remove <system package>

#### 参与贡献

1.  Fork 本仓库 $ git clone https://gitee.com/openeuler/syscare.git
2.  建立自己分支 $ cd syscare & git branch -b Feature_XXXX
3.  完善特性代码 $ vim src/upatch/xxxx  & git commit -m ""
4.  提交代码 $ git push origin
5.  新建 Pull Request


#### 特技

1.  使用 Readme\_XXX.md 来支持不同的语言，例如 Readme\_en.md, Readme\_zh.md
2.  Gitee 官方博客 [blog.gitee.com](https://blog.gitee.com)
3.  你可以 [https://gitee.com/explore](https://gitee.com/explore) 这个地址来了解 Gitee 上的优秀开源项目
4.  [GVP](https://gitee.com/gvp) 全称是 Gitee 最有价值开源项目，是综合评定出的优秀开源项目
5.  Gitee 官方提供的使用手册 [https://gitee.com/help](https://gitee.com/help)
6.  Gitee 封面人物是一档用来展示 Gitee 会员风采的栏目 [https://gitee.com/gitee-stars/](https://gitee.com/gitee-stars/)
