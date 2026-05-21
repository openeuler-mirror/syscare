# SysCare build

SysCare patch creation tool

SysCare build is a CLI tool that generates hot patch packages from RPM packages. The patch packages are encapsulated and maintained as RPM packages. It supports the creation of kernel hot patches and user-mode hot patches.

## Command-Line Parameters

```bash
USAGE:
    syscare build [OPTIONS] --patch-name <PATCH_NAME> --source <SOURCE>... --debuginfo <DEBUGINFO>... --patch <PATCH>...

OPTIONS:
    -n, --patch-name <PATCH_NAME>                  Patch name
        --patch-arch <PATCH_ARCH>                  Patch architecture [default: x86_64]
        --patch-version <PATCH_VERSION>            Patch version [default: 1]
        --patch-release <PATCH_RELEASE>            Patch release [default: 1]
        --patch-description <PATCH_DESCRIPTION>    Patch description [default: (none)]
        --patch-requires <PATCH_REQUIRES>...       Patch requirements
    -s, --source <SOURCE>...                       Source package(s)
    -d, --debuginfo <DEBUGINFO>...                 Debuginfo package(s)
    -p, --patch <PATCH>...                         Patch file(s)
        --build-root <BUILD_ROOT>                  Build directory [default: .]
    -o, --output <OUTPUT>                          Output directory [default: .]
    -j, --jobs <JOBS>                              Parallel build jobs [default: 20]
        --skip-compiler-check                      Skip compiler version check (not recommended)
        --skip-cleanup                             Skip post-build cleanup
    -v, --verbose                                  Provide more detailed info
    -h, --help                                     Print help information
    -V, --version                                  Print version information
```

### Options

|Name|Description|Type|Remarks|
| ---- | ---- | ---- | ---- |
|-n, --patch-name `<PATCH_NAME>`|Patch name|Character string|Mandatory. The name must comply with the RPM naming rules.|
|--patch-arch `<PATCH_ARCH>`|Patch architecture|Character string|The current architecture is used by default. The value must comply with the RPM naming rules.|
|--patch-version `<PATCH_VERSION>`|Patch version|Character string|The default value is 1. The value must comply with the RPM naming rules.|
|--patch-release `<PATCH_RELEASE>`|Patch release|Digits|The default value is 1. The value must comply with the RPM naming rules.|
|--patch-description `<PATCH_DESCRIPTION>`|Patch description|Character string|The description is (none) by default.|
|--patch-requires `<PATCH_REQUIRES>`|Patch dependency|Character string|The description is (none) by default.|
|-s, --source `<SOURCE>`|Path of the source package (**src.rpm**) of the target software|Character string|This parameter is mandatory and must be a valid path.|
|-d, --debuginfo `<DEBUGINFO>`|Path of the **debuginfo** package of the target software|Character string|This parameter is mandatory and must be a valid path.|
|-p, --patch `<PATCH>`|Path of the **debuginfo** package of the target software|Character string|This parameter is mandatory and must be a valid path.|
|--build-root `<BUILD_ROOT>`|Temporary compilation directory|Character string|The default directory is the current execution directory.|
|-o, --output `<OUTPUT>`|Patch output folder|Character string|It must be a valid path and the default directory is the current execution directory.|
|-j, --jobs `<N>`|Number of parallel compilation threads|Digits|The default value is the number of CPU threads.|
|--skip-compiler-check|To skip the compiler check|Flag|-|
|--skip-cleanup|To skip the clearing of temporary files|Flag|-|
|-v, --verbose|To print detailed information|Flag|-|
|-h, --help|To print help information|Flag|-|
|-V, --version|To print version information|Flag|-|

### Return Values

* If the operation is successful, **0** is returned.
* If an error occurs, **255** is returned.

### Outputs

* Patch package: contains the binary and metadata information of SysCare and is used for hot patch installation.
* Patch source package: contains the source code of the target software and new patches, and is used to create hot patches for new versions.

### Naming Rules

* Patch package: **patch-*full_name_of_the_target_software*-*patch_name*-*patch_version*-*patch_release*.*architecture_name*.rpm**

* Patch source package: ***full_name_of_the_target_software*.*patch_name*.*patch_version*.*patch_release*.src.rpm**

### Patch Package Installation Location

```bash
/usr/lib/syscare/patches/${uuid}
```

### Example

```bash
syscare build \
    --patch-name "HP001" \
    --patch-description "CVE-2021-32675 - When parsing an incoming Redis Standard Protocol (RESP) request, Redis allocates memory according to user-specified values which determine the number of elements (in the multi-bulk header) and size of each element (in the bulk header). An attacker delivering specially crafted requests over multiple connections can cause the server to allocate significant amount of memory. Because the same parsing mechanism is used to handle authentication requests, this vulnerability can also be exploited by unauthenticated users." \
    --source ./redis-6.2.5-1.src.rpm \
    --debuginfo ./redis-debuginfo-6.2.5-1.x86_64.rpm \
    --output ./output \
        ./0001-Prevent-unauthenticated-client-from-easily-consuming.patch
```

## Patch Details

The patch metadata contains the following fields.

| Field | Field Description |
| ----------- | ---------------------- |
| uuid | Patch ID |
| name | Patch name |
| version | Patch version |
| release | Patch release |
| arch | Patch architecture |
| type | Patch type |
| target | Target software name |
| license | Target software license |
| description | Patch description |
| entities | Patch entity list |
| patch | Patch file list |

Example:

```bash
dev@dev-x86:[output]$ syscare info redis-6.2.5-1/HP001-1-1
---------------------------------------------------
Patch: redis-6.2.5-1/HP001-1-1
---------------------------------------------------
uuid:        ec503257-aa75-4abc-9045-c4afdd7ae0f2
name:        HP001
version:     1
release:     1
arch:        x86_64
type:        UserPatch
target:      redis-6.2.5-1
license:     BSD and MIT
description: CVE-2021-32675 - When parsing an incoming Redis Standard Protocol (RESP) request, Redis allocates memory according to user-specified values which determine the number of elements (in the multi-bulk header) and size of each element (in the bulk header). An attacker delivering specially crafted requests over multiple connections can cause the server to allocate significant amount of memory. Because the same parsing mechanism is used to handle authentication requests, this vulnerability can also be exploited by unauthenticated users.
entities:
* redis-server
* redis-cli
* redis-benchmark
patch:
* 0001-Prevent-unauthenticated-client-from-easily-consuming.patch
---------------------------------------------------
```

## Patch creation process

1. Prepare the source RPM package of the target software and the debuginfo RPM package.

   Example:

   ```bash
   yumdownloader kernel --source
   yumdownloader kernel-debuginfo
   ```

2. Ensure that the software compilation dependencies are met.

   Example:

   ```bash
   dnf install make gcc bison flex openssl-devel dwarves python3-devel elfutils-libelf-devel
   ```

3. Run the **syscare build** command.

   Example:

   ```bash
   syscare build \
           --patch-name HP001 \
           --source kernel-5.10.0-60.66.0.91.oe2203.src.rpm \
           --debuginfo kernel-debuginfo-5.10.0-60.66.0.91.oe2203.x86_64.rpm \
           --output output \
           001-kernel-patch-test.patch
   ```

   Example:

   ```bash
   dev@dev-x86:[kernel_patch]$ ls -l syscare-build.111602/
   total 100
   -rw-r--r--. 1 dev dev 92303 Nov 12 00:00 build.log
   drwxr-xr-x. 6 dev dev  4096 Nov 12 00:00 package
   drwxr-xr-x. 4 dev dev  4096 Nov 12 00:00 patch
   ```

   The compilation log is generated in the temporary folder named `build.log`.

   ```bash
   dev@dev-x86:[kernel_patch]$ cat syscare-build.111602/build.log | less
   ...
   ```

   If the patch is successfully created, the temporary folder will not be retained.

4. Check the build result.

   Example:

   ```bash
   dev@dev-x86:[output]$ ll
   total 372M
   -rw-r--r--. 1 dev dev 186M Nov 12 00:00 kernel-5.10.0-60.80.0.104.oe2203-HP001-1-1.x86_64.src.rpm
   -rw-r--r--. 1 dev dev  11K Nov 12 00:00 patch-kernel-5.10.0-60.80.0.104.oe2203-HP001-1-1.x86_64.rpm
   ```

   Where:

   `kernel-5.10.0-60.80.0.104.oe2203-HP001-1-1.x86_64.src.rpm` is the patch source package.

   `patch-kernel-5.10.0-60.80.0.104.oe2203-HP001-1-1.x86_64.rpm` is the patch binary package.

## Error Handling

If an error occurs, see the compilation log.

Negative example:

```bash
...
Building patch, this may take a while
ERROR: Process '/usr/libexec/syscare/upatch-build' exited unsuccessfully, exit_code=255
```

## Constraints

1. The patch creation must meet the compilation dependency of the source package.

2. The version of the patch source package must be the same as that of the debuginfo package.

3. The patch package build environment must be the same as the debuginfo package build environment.

4. All the files and folders specified by the parameters must exist.

5. If the input parameters are incorrect, no log will be generated.

6. You are advised to run the command as a non-root user.
