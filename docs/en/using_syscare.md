# Using SysCare

This chapter describes how to use SysCare on openEuler.

## Prerequisites

openEuler 22.03 LTS SP2 has been installed.

## Using SysCare CLI Tools

You can use `syscare build` to create patches and use `syscare patch` to manage patches, including installing, activating, deactivating, confirming, and uninstalling patches.

### Creating Patches

`syscare-build` is used to create patches, for example:

```shell
syscare build \
   --patch-name "HP001" \
   --source ./redis-6.2.5-1.src.rpm \
   --debuginfo ./redis-debuginfo-6.2.5-1.x86_64.rpm \
   --output ./output \
   ./0001-Prevent-unauthenticated-client-from-easily-consuming.patch
```

### Managing Patches

The pattern for matching a patch name is **TARGET_PACKAGE_NAME/PATCH_NAME**. If **PATCH_NAME** is unique, **TARGET_PACKAGE_NAME/** can be omitted. UUIDs can also be used to identify packages.

1. Installing a patch:

    ```shell
    syscare apply PATCH_NAME
    ```

2. Activating a patch:

    ```shell
    syscare active PATCH_NAME
    ```

3. Deactivating a patch:

    ```shell
    syscare deactive PATCH_NAME
    ```

4. Uninstalling/removing a patch:

    ```shell
    syscare remove PATCH_NAME
    ```

5. Confirming a patch:

    ```shell
    syscare accept patch-name
    ```

6. Querying the status of a patch:

    ```shell
    syscare status PATCH_NAME
    ```

7. Querying all SysCare patches:

    ```shell
    syscare list
    ```

## Patch Making Module

### SysCare Patch Making Tool

`syscare-build` is a CLI tool that creates kernel- and user-mode live patches from RPM packages. Patches are encapsulated into RPM packages.

### Command Parameters

```text
Usage: syscare-build [OPTIONS] --patch-name <PATCH_NAME> --source <SOURCE> --debuginfo <DEBUGINFO> <PATCHES>...

Arguments:
  <PATCHES>...  Patch file(s)

Options:
  -n, --patch-name <PATCH_NAME>                Patch name
      --patch-arch <PATCH_ARCH>                Patch architecture [default: x86_64]
      --patch-version <PATCH_VERSION>          Patch version [default: 1]
      --patch-release <PATCH_RELEASE>          Patch release [default: 1]
      --patch-description <PATCH_DESCRIPTION>  Patch description [default: (none)]
      --target-name <TARGET_NAME>              Patch target name
  -t, --target-elfname <TARGET_ELFNAME>        Patch target executable name
      --target-arch <TARGET_ARCH>              parch target architecture
      --target-epoch <TARGET_EPOCH>            Patch target epoch
      --target-version <TARGET_VERSION>        Patch target version
      --target-release <TARGET_RELEASE>        Patch target release
      --target-license <TARGET_LICENSE>        Patch target license
  -s, --source <SOURCE>                        Source package
  -d, --debuginfo <DEBUGINFO>                  Debuginfo package
      --workdir <WORKDIR>                      Working directory [default: .]
  -o, --output <OUTPUT>                        Generated patch output directory [default: .]
  -j, --jobs <N>                               Parallel build jobs [default: 96]
      --skip-compiler-check                    Skip compiler version check (not recommended)
      --skip-cleanup                           Skip post-build cleanup
  -v, --verbose                                Provide more detailed info
  -h, --help                                   Print help information
  -V, --version                                Print version information
```

### Parameters

|Name|Description|Type|Note|
| ---- | ---- | ---- | ---- |
| *\<PATCHES>* |Patch file path|String|Mandatory. The value can be multiple valid paths.|

### Options

|Name|Description|Type|Note|
| ---- | ---- | ---- | ---- |
|-n, --patch-name *\<PATCH_NAME>*|Patch name|String|Mandatory. The value must comply with the RPM package naming convention.|
|--patch-arch *\<PATCH_ARCH>*|Patch architecture|String|The default value is the current architectures. The value must comply with the RPM package naming convention.|
|--patch-version *\<PATCH_VERSION>*|Patch version|String|The default value is **1**. The value must comply with the RPM package naming convention.|
|--patch-release *\<PATCH_RELEASE>*|Patch release|Integer|The default value is **1**. The value must comply with the RPM package naming convention.|
|--patch-description *\<PATCH_DESCRIPTION>*|Patch description|String|The default value is **none**.|
|--target-name *\<TARGET_NAME>*|Target software RPM package name|String|The default value is determined by the **src.rpm** package specified by `--source`.|
|--target-arch *\<TARGET_ARCH>*|Target software RPM package architecture|String|The default value is determined by the **src.rpm** package specified by `--source`.|
|--target-epoch *\<TARGET_EPOCH>*|Target software RPM package epoch|String|The default value is determined by the **src.rpm** package specified by `--source`.|
|--target-version *\<TARGET_VERSION>*|Target software RPM package version|String|The default value is determined by the **src.rpm** package specified by `--source`.|
|--target-release *\<TARGET_RELEASE>*|Target software RPM package release|String|The default value is determined by the **src.rpm** package specified by `--source`.|
|--target-license *\<TARGET_LICENSE>*|Target software RPM package license|String|The default value is determined by the **src.rpm** package specified by `--source`.|
|-s, --source *\<SOURCE>*|Target software **src.rpm** package path|String|Mandatory. The value must be a valid path.|
|-d, --debuginfo *\<DEBUGINFO>*|Target software **debuginfo** package path|String|Mandatory. The value must be a valid path.|
|--workdir *\<WORKDIR>*|Temporary directory|String|The default value is the current directory. The value must be a valid path.|
|-o, --output *\<OUTPUT>*|Patch output directory|String|The default value is the current directory. The value must be a valid path.|
|-j, --jobs *\<N>*|Number of parallel compilation jobs|Integer|The default value is the number of CPU threads|
|--skip-compiler-check|Skip compiler check|Flag|-|
|--skip-cleanup|Skip temporary file cleanup|Flag|-|
|-v, --verbose|Print detail information|Flag|-|
|-h, --help|Print help information|Flag|-|
|-V, --version|Print version information|Flag|-|

An example command is as follows:

```shell
syscare build \
    --patch-name "HP001" \
    --patch-description "CVE-2021-32675 - When parsing an incoming Redis Standard Protocol (RESP) request, Redis allocates memory according to user-specified values which determine the number of elements (in the multi-bulk header) and size of each element (in the bulk header). An attacker delivering specially crafted requests over multiple connections can cause the server to allocate significant amount of memory. Because the same parsing mechanism is used to handle authentication requests, this vulnerability can also be exploited by unauthenticated users." \
    --source ./redis-6.2.5-1.src.rpm \
    --debuginfo ./redis-debuginfo-6.2.5-1.x86_64.rpm \
    --output ./output \
        ./0001-Prevent-unauthenticated-client-from-easily-consuming.patch
```

### Patch Output

- A patch package that contains the binary file of SysCare and meta information. This package is used to install the live patch.
- A patch source package that contains the target software source code and the new patch. This package is used to create live patches for new versions.

Naming rules:

- Patch package: `patch-*TARGET_SOFTWARE_FULL_NAME*-*PATCH_NAME*-*PATCH_VERSION*-*PATCH_RELEASE*.*PATCH_ARCHITECTURE*.rpm`
- Patch source code package: `*TARGET_SOFTWARE_FULL_NAME*-*PATCH_NAME*-*PATCH_VERSION*-*PATCH_RELEASE*.*PATCH_ARCHITECTURE*.src.rpm`

### Patch Information

The patch meta information contains the following fields:

| Field    | Description               |
| ----------- | ---------------------- |
| uuid | Patch ID |
| name        | Patch name               |
| version     | Patch version               |
| release     | Patch release           |
| arch        | Patch architecture               |
| type        | Patch type               |
| target      | Target software name             |
| target_elf | Name of the executable file of the target software |
| digest | Patch fingerprint |
| license     | Target software license          |
| description | Patch description               |
| patch| Patch file list |

Example:

```text
syscare info redis-6.2.5-1/HP001
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

### Patch Making Process

1. Prepare the source package (source RPM) and debugging information package (debuginfo RPM) of the target software.

   Example:

   ```shell
   yumdownloader kernel --source

   yumdownloader kernel --debuginfo
   ```

2. Ensure that the related software build dependencies are installed.

   Example:

   ```shell
   dnf install make gcc bison flex openssl-devel dwarves python3-devel elfutils-libelf-devel
   ```

3. Run the `syscare-build` command.

   Example:

   ```shell
   syscare build \
           --patch-name HP001 \
           --source kernel-5.10.0-60.66.0.91.oe2203.src.rpm \
           --debuginfo kernel-debuginfo-5.10.0-60.66.0.91.oe2203.x86_64.rpm \
           --output output \
           001-kernel-patch-test.patch
   ```

   During patch making, a temporary folder whose name starts with **syscare-build** is created in the directory specified by `--workdir` (the current directory by default) to store temporary files and build logs.

   Example:

   ```shell
   $ ls -l syscare-build.111602/
   total 100
   -rw-r--r--. 1 dev dev 92303 Nov 12 00:00 build.log
   drwxr-xr-x. 6 dev dev 4096 Nov 12 00:00 package
   drwxr-xr-x. 4 dev dev 4096 Nov 12 00:00 patch
   ```

   Build logs (**build.log**) are generated in the temporary folder.

   ```shell
   $ cat syscare-build.111602/build.log | less
   ...
   ```

   If the patch is created successfully and `--skip-compiler-check` is not specified, the temporary folder will be deleted after patch making.

4. Check the build result.

   Example:

   ```shell
   $ ls -l
   total 189680
   -rw-r--r--. 1 dev dev 194218767 Nov 12 00:00 kernel-5.10.0-60.91.0.115.oe2203-HP001-1-1.x86_64.src.rpm
   -rw-r--r--. 1 dev dev     10937 Nov 12 00:00 patch-kernel-5.10.0-60.91.0.115.oe2203-HP001-1-1.x86_64.rpm
   ```

   In the output:

   **patch-kernel-5.10.0-60.91.0.115.oe2203-HP001-1-1.x86_64.rpm** is the patch package.

   **kernel-5.10.0-60.91.0.115.oe2203-HP001-1-1.x86_64.src.rpm** is the patch source package.

5. Install the patch.

   ```shell
   dnf install patch-xxx.rpm
   ```

   After the patch is installed, files in the patch are stored in the **/usr/lib/syscare/patches/target_software_package_name/patch_name** directory

6. Uninstall the patch.

   ```shell
   dnf remove patch-xxx
   ```

   The patch package will be uninstalled when the patch is beyond the **ACTIVED** state.

### Error Handling

If an error occurs, see the build logs:

Error output example:

```text
...
Building patch, this may take a while
ERROR: Process '/usr/libexec/syscare/upatch-build' exited unsuccessfully, exit_code=255
```
