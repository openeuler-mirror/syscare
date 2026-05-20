# SysCare CLI

SysCare CLI Entry

## Format

```bash
syscare [OPTIONS] <COMMAND>
```

## Command `<COMMAND>`

|Name|Description|
| ---- | ---- |
| build | Creates a patch.|
| info | Displays patch information.|
| target | Displays information about the target patch software package.|
| status | Displays the current patch status.|
| list | Displays the patch status list.|
| apply | Loads and activates a patch.|
| remove | Deactivates and uninstalls a patch.|
| active | Activates a patch.|
| deactive | Deactivates a patch.|
| accept | Confirms a patch.|
| save | Saves the status of all patches.|
| restore | Restores the status of all patches.|

### Options

|Name|Description|Type|
| ---- | ---- | ---- |
| -v, --verbose | Prints detailed information.| ID|
| -h, --help | Prints help information.| ID|
| -V, --version | Prints version information.| ID|

## syscare build

See [../builder/README.md](https://gitee.com/openeuler/syscare/blob/master/builder/README.md).

## syscare info

### Description

Displays detailed patch information.

### Constraints

None

### Format

```bash
syscare info <IDENTIFIER>
```

### Parameters

|Name|Description|Type|Remarks|
| ---- | ---- | ---- | ---- |
|`<IDENTIFIER>`|Patch name|Character string|The value can be `target_name/patch_name` or `uuid`.|

### Options

|Name|Description|Type|
| ---- | ---- | ---- |
| -h, --help | Prints help information.| ID|

### Return Values

* If the operation is successful, **0** is returned.
* If an error occurs, **255** is returned.

### Example

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

### Description

Displays information about the target patch software package.

### Constraints

None

### Format

```bash
syscare target <IDENTIFIER>
```

### Parameters

|Name|Description|Type|Remarks|
| ---- | ---- | ---- | ---- |
|`<IDENTIFIER>`|Patch name|Character string|The value can be `target_name/patch_name` or `uuid`.|

### Options

|Name|Description|Type|
| ---- | ---- | ---- |
| -h, --help | Prints help information.| ID|

### Return Values

* If the operation is successful, **0** is returned.
* If an error occurs, **255** is returned.

### Example

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

### Description

Displays the current patch status.

### Constraints

None

### Format

```bash
syscare target <IDENTIFIER>
```

### Parameters

|Name|Description|Type|Remarks|
| ---- | ---- | ---- | ---- |
|`<IDENTIFIER>`|Patch name|Character string|The value can be `patch_name`, `target_name/patch_name`, or `uuid`.|

### Options

|Name|Description|Type|
| ---- | ---- | ---- |
| -h, --help | Prints help information.| ID|

### Return Values

* If the operation is successful, **0** is returned.
* If an error occurs, **255** is returned.

### Example

```bash
dev@openeuler-dev:[/]$ syscare status redis-6.2.5-1/HP001
ACTIVED
```

## syscare list

### Description

Displays the patch status list.

### Constraints

None

### Format

```bash
syscare list
```

### Parameters

None

### Options

|Name|Description|Type|
| ---- | ---- | ---- |
| -h, --help | Prints help information.| ID|

### Return Values

* If the operation is successful, **0** is returned.
* If an error occurs, **255** is returned.

### Example

```bash
dev@openeuler-dev:[/]$ syscare list
Uuid                                     Name                                     Status
ec503257-aa75-4abc-9045-c4afdd7ae0f2     redis-6.2.5-1/HP001                      ACTIVED
28f35f80-a0b8-4a89-9172-9c0705a95ab0     redis-6.2.5-1/HP002                      NOT-APPLIED
6a5735b6-496f-40ab-a92c-2ab32761851d     nginx-1.21.5-4/HP001                     NOT-APPLIED
b6bf2bf3-ddeb-4e8d-b8fe-a86971b1c62c     kernel-5.10.0-60.80.0.104.oe2203/HP001   NOT-APPLIED
```

## syscare apply

### Description

Loads and activates a patch. After the operation is successful, the patch enters the `ACTIVED` state.

If the patch has been loaded, skip the activation step and a message will be displayed.

### Constraints

The **root** permission is required.

### Format

```bash
syscare apply <IDENTIFIER>
```

### Parameters

|Name|Description|Type|Remarks|
| ---- | ---- | ---- | ---- |
|`<IDENTIFIER>`|Patch name|Character string| The value can be `target_name/patch_name` or `uuid`.|

### Options

|Name|Description|Type|
| ---- | ---- | ---- |
| -h, --help | Prints help information.| ID|

### Return Values

* If the operation is successful, **0** is returned.
* If an error occurs, **255** is returned.

### Example

```bash
dev@openeuler-dev:[/]$ sudo syscare apply redis-6.2.5-1/HP001
dev@openeuler-dev:[/]$
```

## syscare remove

### Description

Deactivates and uninstalls a patch. After the operation is successful, the patch enters the `NOT-APPLIED` state.

### Constraints

The **root** permission is required.

### Format

```bash
syscare remove <IDENTIFIER>
```

### Parameters

|Name|Description|Type|Remarks|
| ---- | ---- | ---- | ---- |
|`<IDENTIFIER>`|Patch name|Character string|The value can be `target_name/patch_name` or `uuid`.|

### Options

|Name|Description|Type|
| ---- | ---- | ---- |
| -h, --help | Prints help information.| ID|

### Return Values

* If the operation is successful, **0** is returned.
* If an error occurs, **255** is returned.

### Example

```bash
dev@openeuler-dev:[/]$ sudo syscare remove redis-6.2.5-1/HP001
dev@openeuler-dev:[/]$
```

## syscare active

### Description

Activates a patch. After the operation is successful, the patch enters the `ACTIVED` state.

### Constraints

The **root** permission is required.

### Format

```bash
syscare active <IDENTIFIER>
```

### Parameters

|Name|Description|Type|Remarks|
| ---- | ---- | ---- | ---- |
|`<IDENTIFIER>`|Patch name|Character string|The value can be `target_name/patch_name` or `uuid`.|

### Options

|Name|Description|Type|
| ---- | ---- | ---- |
| -h, --help | Prints help information.| ID|

### Return Values

* If the operation is successful, **0** is returned.
* If an error occurs, **255** is returned.

### Example

```bash
dev@openeuler-dev:[/]$ sudo syscare active redis-6.2.5-1/HP001
dev@openeuler-dev:[/]$
```

## syscare deactive

### Description

Deactivates a patch. After the operation is successful, the patch enters the `DEACTIVED` state.

### Constraints

The **root** permission is required.

### Format

```bash
syscare deactive <IDENTIFIER>
```

### Parameters

|Name|Description|Type|Remarks|
| ---- | ---- | ---- | ---- |
|`<IDENTIFIER>`|Patch name|Character string|The value can be `target_name/patch_name` or `uuid`.|

### Options

|Name|Description|Type|
| ---- | ---- | ---- |
| -h, --help | Prints help information.| ID|

### Return Values

* If the operation is successful, **0** is returned.
* If an error occurs, **255** is returned.

### Example

```bash
dev@openeuler-dev:[/]$ sudo syscare deactive redis-6.2.5-1/HP001
dev@openeuler-dev:[/]$
```

## syscare accept

### Description

Confirms the patch status. After the operation is successful, the patch enters the `ACCEPT` state and is reapplied after the system restarts.

### Constraints

The **root** permission is required.

### Format

```bash
syscare accept <IDENTIFIER>
```

### Parameters

|Name|Description|Type|Remarks|
| ---- | ---- | ---- | ---- |
|`<IDENTIFIER>`|Patch name|Character string|The value can be `target_name/patch_name` or `uuid`.|

### Options

|Name|Description|Type|
| ---- | ---- | ---- |
| -h, --help | Prints help information.| ID|

### Return Values

* If the operation is successful, **0** is returned.
* If an error occurs, **255** is returned.

### Example

```bash
dev@openeuler-dev:[/]$ sudo syscare accept redis-6.2.5-1/HP001
dev@openeuler-dev:[/]$
```

## syscare save

### Description

Saves the status of all patches.

### Constraints

The **root** permission is required.

### Format

```bash
syscare save
```

### Parameters

None

### Options

|Name|Description|Type|
| ---- | ---- | ---- |
| -h, --help | Prints help information.| ID|

### Return Values

* If the operation is successful, **0** is returned.
* If an error occurs, **255** is returned.

### Example

```bash
dev@openeuler-dev:[/]$ sudo syscare save
dev@openeuler-dev:[/]$
```

## syscare restore

### Description

Restores the status of all patches.

### Constraints

The **root** permission is required.

### Format

```bash
syscare restore
```

### Parameters

None

### Options

|Name|Description|Type|
| ---- | ---- | ---- |
| --accepted | Restores only patches in the `ACCEPTED` state.| ID|
| -h, --help | Prints help information.| ID|

### Return Values

* If the operation is successful, **0** is returned.
* If an error occurs, **255** is returned.

### Notes

* This command performs the `REMOVE/DEACTIVE` operation first and then the `APPLY/ACTIVE` operation.
* The `DEACTIVE` state will be processed as the `NOT-APPLIED` state.
* The newly discovered (installed) patches will be processed as the `NOT-APPLIED` state.

### Example

```bash
dev@openeuler-dev:[/]$ sudo syscare restore
dev@openeuler-dev:[/]$
```
