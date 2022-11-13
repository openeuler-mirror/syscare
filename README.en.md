# SysCare: System Live Care Service

#### Description
&nbsp;&nbsp;&nbsp;&nbsp;SysCare is a system-level hot-fix software that provides single-machine-level and cluster-level security patches and system error hot-fixes for the operating system. The host can fix the system problem without rebooting. <br />
&nbsp;&nbsp;&nbsp;&nbsp;Currently, only the unified kernel-mode/user-mode hot patch technology is integrated. Users need to focus on their core business, and leave system repair problems to SysCare for processing. In the later stage, according to the different components to be repaired, a system hot upgrade technology will be provided to further liberate the operation and maintenance users and improve the operation and maintenance efficiency.

#### Software Architecture
&nbsp;&nbsp;&nbsp;&nbsp;SysCare can use the source code of system components and the corresponding patch problems to produce RPMs for corresponding component patches (including patch files, dependency information and configuration information, etc.). The produced patch RPMs can be uploaded to the corresponding patch warehouses and clustered systems. Demond regularly queries the patch repository, and hot-fixes CVEs and software errors running in the system to ensure safe, stable, and efficient operation of the system.

#### Installation

1.  syscare build <system package>.src.rpm xxxx.patch
2.  syscare apply <system pacakge>-patch.rpm
3.  syscare active <system package>
4.  syscare deactive <system packge>
5.  syscare remove <system package>

#### Instructions

1.  Clone syscare $ git clone https://gitee.com/openeuler/syscare.git
2.  Create develop branch $ cd syscare & git branch -b Feature_XXXX
3.  Complete your feature $ vim src/upatch/xxxx  & git commit -m ""
4.  Push your code $ git push origin
5.  Issue Pull Request

#### Contribution

1.  Fork the repository
2.  Create Feat_xxx branch
3.  Commit your code
4.  Create Pull Request


#### Gitee Feature

1.  You can use Readme\_XXX.md to support different languages, such as Readme\_en.md, Readme\_zh.md
2.  Gitee blog [blog.gitee.com](https://blog.gitee.com)
3.  Explore open source project [https://gitee.com/explore](https://gitee.com/explore)
4.  The most valuable open source project [GVP](https://gitee.com/gvp)
5.  The manual of Gitee [https://gitee.com/help](https://gitee.com/help)
6.  The most popular members  [https://gitee.com/gitee-stars/](https://gitee.com/gitee-stars/)
