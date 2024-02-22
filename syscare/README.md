# Syscare CLI
Syscare cli入口

## 调用格式
```bash
syscare [OPTIONS] <COMMAND>
```
## 命令``` <COMMAND>```

|名称|描述|
| ---- | ---- |
| build | 制作补丁 |
| info | 查看补丁信息 |
| target | 查看补丁目标软件包信息 |
| status | 查看补丁当前状态 |
| list | 查看补丁状态列表 |
| apply | 加载并激活补丁 |
| remove | 去激活并卸载补丁 |
| active | 激活补丁 |
| deactive | 去激活补丁 |
| accept | 确认补丁 |
| save | 保存所有补丁状态 |
| restore | 恢复所有补丁状态 |

### 选项
|名称|描述|类型|
| ---- | ---- | ---- |
| -v, --verbose | 打印详细信息 | 标识 |
| -h, --help | 打印帮助信息 | 标识 |
| -V, --version | 打印版本信息 | 标识 |



## syscare build
请见[../builder/README.md](https://gitee.com/openeuler/syscare/blob/master/builder/README.md)



## syscare info

### 说明
显示补丁详细信息

### 约束
无

### 调用格式
```bash
syscare info <IDENTIFIER>
```

### 参数
|名称|描述|类型|备注|
| ---- | ---- | ---- | ---- |
|```<IDENTIFIER>```|补丁名称|字符串|可为```target_name/patch_name```，也可为```uuid```|

### 选项
|名称|描述|类型|
| ---- | ---- | ---- |
| -h, --help | 打印帮助信息 | 标识 |

### 返回值
* 成功返回 0
* 错误返回255

### 示例
```bash
dev@openeuler-dev:[/]$ syscare info redis-6.2.5-1/HP001
uuid:        ec503257-aa75-4abc-9045-c4afdd7ae0f2
name:        HP001
version:     1
release:     31fc7544
arch:        x86_64
type:        UserPatch
target:      redis-6.2.5-1
target_elf:  redis-server
digest:      31fc7544
license:     BSD and MIT
description: CVE-2021-32675 - When parsing an incoming Redis Standard Protocol (RESP) request, Redis allocates memory according to user-specified values which determine the number of elements (in the multi-bulk header) and size of each element (in the bulk header). An attacker delivering specially crafted requests over multiple connections can cause the server to allocate significant amount of memory. Because the same parsing mechanism is used to handle authentication requests, this vulnerability can also be exploited by unauthenticated users.
patch:
31fc7544 0001-Prevent-unauthenticated-client-from-easily-consuming.patch
```



## syscare target

### 说明
查看补丁目标软件包信息

### 约束
无

### 调用格式
```bash
syscare target <IDENTIFIER>
```

### 参数
|名称|描述|类型|备注|
| ---- | ---- | ---- | ---- |
|```<IDENTIFIER>```|补丁名称|字符串|可为```target_name/patch_name```，也可为```uuid```|

### 选项
|名称|描述|类型|
| ---- | ---- | ---- |
| -h, --help | 打印帮助信息 | 标识 |

### 返回值
* 成功返回 0
* 错误返回255

### 示例
```bash
dev@openeuler-dev:[/]$ syscare target redis-6.2.5-1/HP001
name:    redis
arch:    x86_64
epoch:   (none)
version: 6.2.5
release: 1
license: BSD and MIT
```



## syscare status

### 说明
查看补丁当前状态

### 约束
无

### 调用格式
```bash
syscare target <IDENTIFIER>
```

### 参数
|名称|描述|类型|备注|
| ---- | ---- | ---- | ---- |
|```<IDENTIFIER>```|补丁名称|字符串|可为```patch_name```, 可为```target_name/patch_name```，也可为```uuid```|

### 选项
|名称|描述|类型|
| ---- | ---- | ---- |
| -h, --help | 打印帮助信息 | 标识 |

### 返回值
* 成功返回 0
* 错误返回255

### 示例
```bash
dev@openeuler-dev:[/]$ syscare status redis-6.2.5-1/HP001
ACTIVED
```



## syscare list

### 说明
查看补丁状态列表

### 约束
无

### 调用格式
```bash
syscare list
```
### 参数
无

### 选项
|名称|描述|类型|
| ---- | ---- | ---- |
| -h, --help | 打印帮助信息 | 标识 |

### 返回值
* 成功返回 0
* 错误返回255

### 示例
```bash
dev@openeuler-dev:[/]$ syscare list
Uuid                                     Name                                     Status
ec503257-aa75-4abc-9045-c4afdd7ae0f2     redis-6.2.5-1/HP001                      ACTIVED
28f35f80-a0b8-4a89-9172-9c0705a95ab0     redis-6.2.5-1/HP002                      NOT-APPLIED
6a5735b6-496f-40ab-a92c-2ab32761851d     nginx-1.21.5-4/HP001                     NOT-APPLIED
b6bf2bf3-ddeb-4e8d-b8fe-a86971b1c62c     kernel-5.10.0-60.80.0.104.oe2203/HP001   NOT-APPLIED
```



## syscare apply

### 说明
加载并激活补丁，操作成功后补丁将会转为```ACTIVED```状态

若补丁已加载，则跳过激活步骤打印提示

### 约束
需要root权限

### 调用格式
```bash
syscare apply <IDENTIFIER>
```

### 参数
|名称|描述|类型|备注|
| ---- | ---- | ---- | ---- |
|```<IDENTIFIER>```|补丁名称|字符串| 可为```target_name/patch_name```，也可为```uuid``` |

### 选项
|名称|描述|类型|
| ---- | ---- | ---- |
| -h, --help | 打印帮助信息 | 标识 |

### 返回值
* 成功返回 0
* 错误返回255

### 示例
```bash
dev@openeuler-dev:[/]$ sudo syscare apply redis-6.2.5-1/HP001
dev@openeuler-dev:[/]$
```



## syscare remove

### 说明
去激活并卸载补丁，操作成功后补丁将会转为```NOT-APPLIED```状态

### 约束
需要root权限

### 调用格式
```bash
syscare remove <IDENTIFIER>
```

### 参数
|名称|描述|类型|备注|
| ---- | ---- | ---- | ---- |
|```<IDENTIFIER>```|补丁名称|字符串|可为```target_name/patch_name```，也可为```uuid```|

### 选项
|名称|描述|类型|
| ---- | ---- | ---- |
| -h, --help | 打印帮助信息 | 标识 |

### 返回值
* 成功返回 0
* 错误返回255

### 示例
```bash
dev@openeuler-dev:[/]$ sudo syscare remove redis-6.2.5-1/HP001
dev@openeuler-dev:[/]$
```



## syscare active

### 说明
激活补丁，操作成功后补丁将会转为```ACTIVED```状态

### 约束
需要root权限

### 调用格式
```bash
syscare active <IDENTIFIER>
```

### 参数
|名称|描述|类型|备注|
| ---- | ---- | ---- | ---- |
|```<IDENTIFIER>```|补丁名称|字符串|可为```target_name/patch_name```，也可为```uuid```|

### 选项
|名称|描述|类型|
| ---- | ---- | ---- |
| -h, --help | 打印帮助信息 | 标识 |

### 返回值
* 成功返回 0
* 错误返回255

### 示例
```bash
dev@openeuler-dev:[/]$ sudo syscare active redis-6.2.5-1/HP001
dev@openeuler-dev:[/]$
```



## syscare deactive

### 说明
去激活补丁，操作成功后补丁将会转为```DEACTIVED```状态

### 约束
需要root权限

### 调用格式
```bash
syscare deactive <IDENTIFIER>
```

### 参数
|名称|描述|类型|备注|
| ---- | ---- | ---- | ---- |
|```<IDENTIFIER>```|补丁名称|字符串|可为```target_name/patch_name```，也可为```uuid```|

### 选项
|名称|描述|类型|
| ---- | ---- | ---- |
| -h, --help | 打印帮助信息 | 标识 |

### 返回值
* 成功返回 0
* 错误返回255

### 示例
```bash
dev@openeuler-dev:[/]$ sudo syscare deactive redis-6.2.5-1/HP001
dev@openeuler-dev:[/]$
```


## syscare accept

### 说明
确认补丁状态，操作成功后补丁将会转为```ACCEPT```状态，并在系统重启后重新应用

### 约束
需要root权限

### 调用格式
```bash
syscare accept <IDENTIFIER>
```

### 参数
|名称|描述|类型|备注|
| ---- | ---- | ---- | ---- |
|```<IDENTIFIER>```|补丁名称|字符串|可为```target_name/patch_name```，也可为```uuid```|

### 选项
|名称|描述|类型|
| ---- | ---- | ---- |
| -h, --help | 打印帮助信息 | 标识 |

### 返回值
* 成功返回 0
* 错误返回255

### 示例
```bash
dev@openeuler-dev:[/]$ sudo syscare accept redis-6.2.5-1/HP001
dev@openeuler-dev:[/]$
```


## syscare save

### 说明
保存所有补丁状态

### 约束
需要root权限

### 调用格式
```bash
syscare save
```

### 参数
无

### 选项
|名称|描述|类型|
| ---- | ---- | ---- |
| -h, --help | 打印帮助信息 | 标识 |

### 返回值
* 成功返回 0
* 错误返回255

### 示例
```bash
dev@openeuler-dev:[/]$ sudo syscare save
dev@openeuler-dev:[/]$
```



## syscare restore

### 说明
恢复所有补丁状态

### 约束
需要root权限

### 调用格式
```bash
syscare restore
```

### 参数
无

### 选项
|名称|描述|类型|
| ---- | ---- | ---- |
| --accepted | 仅恢复状态为```ACCEPTED```的补丁 | 标识 |
| -h, --help | 打印帮助信息 | 标识 |

### 返回值
* 成功返回 0
* 错误返回255

### 提示
* 该命令会先进行```REMOVE/DEACTIVE```操作，再进行```APPLY/ACTIVE```操作
* ```DEACTIVE```状态将会被当作```NOT-APPLIED```状态处理
* 新发现（安装）的补丁将会被当作```NOT-APPLIED```状态处理

### 示例

```bash
dev@openeuler-dev:[/]$ sudo syscare restore
dev@openeuler-dev:[/]$
```
