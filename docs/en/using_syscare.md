# Using SysCare

This chapter describes how to use SysCare on openEuler.

## Prerequisites

openEuler 24.03 LTS SP1 has been installed.

## SysCare Usage

This chapter describes how to use SysCare, covering both hot patch creation and management.

### Hot Patch Creation

Users can create hot patches using the `sycare build` command. This command is a pure CLI tool that generates hot patch packages from RPM packages. The hot patch packages are packaged and maintained as RPM packages and support the creation of both kernel and user-space hot patches.

#### Hot Patch Creation Process

1. Prepare the source RPM package and the debuginfo RPM package for the target software.

   Example:

   ```shell
   yumdownloader kernel --source --debuginfo
   ```

2. Ensure that the necessary compilation dependencies for the corresponding software are met.

   Example:

   ```shell
   dnf install make gcc bison flex openssl-devel dwarves python3-devel elfutils-libelf-devel
   ```

3. Execute the `syscare build` command to build the hot patch.

   Example:

   ```shell
   syscare build \
           --patch_name HP001 \
           --source kernel-5.10.0-60.66.0.91.oe2203.src.rpm \
           --debuginfo kernel-debuginfo-5.10.0-60.66.0.91.oe2203.x86_64.rpm \
           --output output \
           --patch 001-kernel-patch-test.patch
   ```

   During the hot patch creation process, a temporary folder starting with `syscare-build` will be created in the directory specified by the `--workdir` parameter (defaulting to the current directory). This folder will store temporary files and compilation logs.

   Example:

   ```shell
   dev@openeuler-dev:[~]$ ls -l syscare-build.111602/
   total 100
   -rw-r--r--. 1 dev dev 92303 Nov 12 00:00 build.log
   drwxr-xr-x. 6 dev dev 4096 Nov 12 00:00 package
   drwxr-xr-x. 4 dev dev 4096 Nov 12 00:00 patch
   ```

   The compilation log, named `build.log`, will be generated within the temporary folder.

   ```shell
   dev@openeuler-dev:[~]$ cat syscare-build.111602/build.log | less
   ```

   If the patch is successfully created and the `--skip-compiler-check` parameter is not used, the temporary folder will be automatically removed.

4. Check the build results.

   Example:

   ```shell
   dev@openeuler-dev:[~]$ ls -l
   total 189680
   -rw-r--r--. 1 dev dev 194218767 Nov 12 00:00 kernel-5.10.0-60.91.0.115.oe2203-HP001-1-1.x86_64.src.rpm
   -rw-r--r--. 1 dev dev     10937 Nov 12 00:00 patch-kernel-5.10.0-60.91.0.115.oe2203-HP001-1-1.x86_64.rpm
   ```

   Here:

   - `patch-kernel-5.10.0-60.91.0.115.oe2203-HP001-1-1.x86_64.rpm` is the patch package.
   - `kernel-5.10.0-60.91.0.115.oe2203-HP001-1-1.x86_64.src.rpm` is the source RPM package.

#### Hot Patch Creation Tool

```shell
USAGE:
    syscare build [OPTIONS] --patch_name <PATCH_NAME> --source <SOURCE> --debuginfo <DEBUGINFO>... --patch <PATCH_FILES>...

OPTIONS:
    -n, --patch_name <PATCH_NAME>                  Patch name
        --patch_arch <PATCH_ARCH>                  Patch architecture [default: x86_64]
        --patch_version <PATCH_VERSION>            Patch version [default: 1]
        --patch_release <PATCH_RELEASE>            Patch release [default: 1]
        --patch_description <PATCH_DESCRIPTION>    Patch description [default: (none)]
    -s, --source <SOURCE>                          Source package
    -d, --debuginfo <DEBUGINFO>...                 Debuginfo package(s)
        --workdir <WORKDIR>                        Working directory [default: .]
    -o, --output <OUTPUT>                          Output directory [default: .]
    -j, --jobs <JOBS>                              Parllel build jobs [default: 96]
        --skip_compiler_check                      Skip compiler version check (not recommended)
        --skip_cleanup                             Skip post-build cleanup
    -v, --verbose                                  Provide more detailed info
    -p, --patch <PATCH_FILES>...                   Patch file(s)
    -h, --help                                     Prints help information
    -V, --version                                  Prints version information
```

| Name                                      | Description                                            | Type   | Notes                                                                          |
| ----------------------------------------- | ------------------------------------------------------ | ------ | ------------------------------------------------------------------------------ |
| -n, --patch_name `<PATCH_NAME>`           | Patch name                                             | String | Required parameter; must comply with RPM naming conventions.                   |
| --patch_arch `<PATCH_ARCH>`               | Patch architecture                                     | String | Defaults to the current architecture; must comply with RPM naming conventions. |
| --patch_version `<PATCH_VERSION>`         | Patch version number                                   | String | Default value is 1; must comply with RPM naming conventions.                   |
| --patch_release `<PATCH_RELEASE>`         | Patch release                                          | Number | Default value is 1; must comply with RPM naming conventions.                   |
| --patch_description `<PATCH_DESCRIPTION>` | Patch description                                      | String | Defaults to (none).                                                            |
| -s, --source `<SOURCE>`                   | Path to the target software's src.rpm source package.  | String | Required parameter; must be a valid path.                                      |
| -d, --debuginfo `<DEBUGINFO>...`          | Path(s) to the target software's debuginfo package(s). | String | Required parameter; multiple can be specified; must be a valid path.           |
| --workdir `<WORKDIR>`                     | Path to the temporary working directory.               | String | Defaults to the current execution directory; must be a valid path.             |
| -o, --output `<OUTPUT>`                   | Output folder for the generated patch.                 | String | Defaults to the current execution directory; must be a valid path.             |
| -j, --jobs `<JOBS>`                       | Number of parallel compilation threads.                | Number | Defaults to the number of CPU threads.                                         |
| --skip-compiler-check                     | Skip the compiler version check (not recommended).     | Flag   | -                                                                              |
| --skip-cleanup                            | Skip the cleanup of temporary files after the build.   | Flag   | -                                                                              |
| -v, --verbose                             | Print detailed information.                            | Flag   | -                                                                              |
| -p, --patch `<PATCHES>...`                | Path(s) to the patch file(s).                          | String | Required parameter; multiple can be specified; must be a valid path.           |
| -h, --help                                | Print help information.                                | Flag   | -                                                                              |
| -V, --version                             | Print version information.                             | Flag   | -                                                                              |
|                                           |                                                        |        |                                                                                |

Example:

```shell
syscare build \
    --patch_name \"HP001\" \\
        --patch_description \"CVE-2021-32675 - When parsing an incoming Redis Standard Protocol (RESP) request, Redis allocates memory according to user-specified values which determine the number of elements (in the multi-bulk header) and size of each element (in the bulk header). An attacker delivering specially crafted requests over multiple connections can cause the server to allocate significant amount of memory. Because the same parsing mechanism is used to handle authentication requests, this vulnerability can also be exploited by unauthenticated users.\" \\
        --source ./redis-6.2.5-1.src.rpm \\
        --debuginfo ./redis-debuginfo-6.2.5-1.x86_64.rpm \\
        --output ./output \\
        --patch ./0001-Prevent-unauthenticated-client-from-easily-consuming.patch
```

#### Hot Patch Package Naming Convention

- Hot patch package: `patch-<target software full name>-<patch name>-<patch version>-<release>.<architecture>.rpm`
- Hot patch source package: `<target software full name>-<patch name>-<patch version>-<release>.<architecture>.src.rpm`

#### Package Contents

- Hot patch package: Contains SysCare hot patch binaries and metadata for patch installation
- Hot patch source package: Includes target software source code and additional patch files for creating new hot patch versions

#### Troubleshooting

If errors occur during hot patch creation, check the compilation log named `build.log` in the working directory.

Example:

```shell
Building patch, this may take a while
- Preparing build requirements
- Building patch
Error: UserPatchBuilder: Failed to build patch

Caused by:
    Process "/usr/libexec/syscare/upatch-build" exited unsuccessfully, exit_code=253
For more information, please check "/home/dev/syscare-build.345549/build.log"
```

### Hot Patch Package Management

Installing or uninstalling hot patches requires specifying the corresponding RPM package name, denoted here as `$patch_package`.

1. Installing a hot patch package

   ```shell
   dnf install $patch_package.rpm
   ```

   After installation, hot patch files are stored in:
   /usr/lib/syscare/patches

2. Uninstalling a hot patch package

   ```shell
   dnf remove $patch_package
   ```

   Note: Hot patches in `ACTIVED` state or higher will be automatically uninstalled.

### Hot Patch Management

The `syscare` command is used to manage hot patches.

To operate on a specific hot patch, users must provide a search string (denoted as `$patch_identifier`).

Search follows these rules: `<target package name>/<patch name>`. The `<target package name>/` prefix can be omitted if the patch name is unique. UUID can also be used for management.

- Target package name: Name of the software package to be patched
- Patch name: Name of the hot patch

#### Patch Metadata

Patch metadata includes the following fields:

| Field       | Description               |
| ----------- | ------------------------- |
| uuid        | Unique patch identifier   |
| name        | Patch name                |
| version     | Version number            |
| release     | Release number            |
| arch        | Architecture              |
| type        | Patch type                |
| target      | Target software name      |
| entities    | Executable files affected |
| digest      | Cryptographic fingerprint |
| license     | Software license          |
| description | Patch details             |
| patch       | List of patch files       |

### Hot Patch Package Management

Installing or uninstalling hot patches requires specifying the corresponding RPM package name, denoted here as `$patch_package`.

1. Installing a hot patch package

   ```shell
   dnf install $patch_package.rpm
   ```

   After installation, hot patch files are stored in:
   /usr/lib/syscare/patches

2. Uninstalling a hot patch package

   ```shell
   dnf remove $patch_package
   ```

   Note: Hot patches in `ACTIVED` state or higher will be automatically uninstalled.

### Hot Patch Management

The `syscare` command is used to manage hot patches.

To operate on a specific hot patch, users must provide a search string (denoted as `$patch_identifier`).

Search follows these rules: `<target package name>/<patch name>`. The `<target package name>/` prefix can be omitted if the patch name is unique. UUID can also be used for management.

- Target package name: Name of the software package to be patched
- Patch name: Name of the hot patch

#### Patch Metadata

Patch metadata includes the following fields:

| Field       | Description               |
| ----------- | ------------------------- |
| uuid        | Unique patch identifier   |
| name        | Patch name                |
| version     | Version number            |
| release     | Release number            |
| arch        | Architecture              |
| type        | Patch type                |
| target      | Target software name      |
| entities    | Executable files affected |
| digest      | Cryptographic fingerprint |
| license     | Software license          |
| description | Patch details             |
| patch       | List of patch files       |

Example:

```shell
sudo syscare info redis-6.2.5-1/HP002-1-1
uuid:        980fa0d0-e753-447c-8494-01de595f35d0
name:        HP002
version:     1
release:     1
arch:        x86_64
type:        UserPatch
target:      redis-6.2.5-1
target_elf:  redis-server, redis-benchmark, redis-cli
license:     BSD and MIT
description: CVE-2021-32675 - When parsing an incoming Redis Standard Protocol (RESP) request, Redis allocates memory according to user-specified values which determine the number of elements (in the multi-bulk header) and size of each element (in the bulk header). An attacker delivering specially crafted requests over multiple connections can cause the server to allocate significant amount of memory. Because the same parsing mechanism is used to handle authentication requests, this vulnerability can also be exploited by unauthenticated users.
patch:
0001-Prevent-unauthenticated-client-from-easily-consuming.patch
```

#### Hot Patch States

SysCare categorizes the hot patch lifecycle into these states:

- Unloaded: `NOT-APPLIED`
- Inactive: `DEACTIVED`
- Active: `ACTIVED`
- Accepted: `ACCEPTED`

#### Patch Information Queries

1. View basic patch information:

   ```shell
   syscare info $patch_identifier
   ```

2. Check patch status:

   ```shell
   syscare status $patch_identifier
   ```

3. List all patch statuses:

   ```shell
   syscare list
   ```

#### Hot Patch State Management

1. Load a patch:

   ```shell
   syscare apply $patch_identifier
   ```

2. Unload a patch:

   ```shell
   syscare remove $patch_identifier
   ```

3. Activate a patch:

   ```shell
   syscare active $patch_identifier
   ```

4. Deactivate a patch:

   ```shell
   syscare deactive $patch_identifier
   ```

5. Accept a patch:

   ```shell
   syscare accept $patch_identifier
   ```

6. Save all patch states:

   ```shell
   syscare save
   ```

7. Restore all patch states:

   ```shell
   syscare restore
   ```
