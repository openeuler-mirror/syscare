# SysCare: System Hot Service

## Overview

​SysCare is a system-level hot repair software that provides stand-alone and cluster-level security patches and system error hot fixes for the operating system. The host can repair system problems without restarting.
​Currently, SysCare combines kernel-mode and user-mode hot patching to take over system repair, freeing up valuable time for users to focus on core services. In the future, the system hot upgrade technology will be provided based on the different components to be repaired, further reducing manual overhead and improving efficiency for O&M teams.

## Software Architecture

​Based on source code of system components and problems to be solved, you can create RPM patch packages for the components. A package contains the patch file, dependency information, and configuration information. The RPM patch packages can be uploaded to the patch repository. The cluster daemon periodically queries the patch repository and performs hot fixing on CVEs and software errors in the system, ensuring a secure, stable, and efficient system.

## Installation

### Installation Using DNF

1. Correctly configure the DNF/YUM repository file.
2. Run the following commands: `dnf update & dnf install syscare`
3. Enjoy the tool.

### Installation by Compiling Source Code

  * Compilation dependency installation

    ```bash
    $ kernel-version=$(uname -r)
    $ dnf install -y kernel-source-$kernel-version kernel-debuginfo-$kernel-version kernel-devel-$kernel-version
    $ dnf install -y elfutils-libelf-devel openssl-devel dwarves python3-devel rpm-build bison cmake make gcc g++
    ```

  * Compilation and installation

    Note: If you directly compile the code, a message indicating that dependencies are missing will be displayed during patch installation. Therefore, you are advised to install the app using the RPM package.

    ```bash
    git clone https://gitee.com/openeuler/syscare.git
    cd syscare
    mkdir tmp
    cd tmp
    cmake -DCMAKE_INSTALL_PREFIX=/usr -DKERNEL_VERSION=$(uname -r) ..
    make
    make install

    mkdir -p /usr/lib/syscare/patches
    systemctl daemon-reload
    systemctl enable syscare
    systemctl start syscare
    ```

  * Offline compilation
    Run the **cargo vendor** command in the network environment to download all dependencies to the **./vendor** directory.

    ```sh
    cd syscare
    cargo vendor
    ```

    Create the **.cargo/config.toml** file in the source code directory and write the following settings to the file. In this way, the network connection is not required for the next compilation.

    ```sh
    [source.crates-io]
    replace-with = "vendored-sources"

    [source.vendored-sources]
    directory = "vendor"
    ```

### Installation Using an RPM Package

```bash
rpm -ivh syscare-*.rpm
```

Or:

```bash
dnf install syscare-*.rpm
```

## Usage

### Patch Creation

```bash
$ syscare build \
   --patch-name "HP001" \
   --patch-description "CVE-2021-32675" \
   --source ./redis-6.2.5-1.src.rpm \
   --debuginfo ./redis-debuginfo-6.2.5-1.x86_64.rpm \
   --output ./output \
   --patch ./0001-Prevent-unauthenticated-client-from-easily-consuming.patch
```

### Kernel Module Hot Patch Creation

```bash
$ syscare build \
   --patch-name HP001 \
   --source ./kernel-5.10.0-60.91.0.115.src.rpm \
   --source ./testmod-1-1.src.rpm \
   --debuginfo ./kernel-debuginfo-5.10.0-60.91.0.115.aarch64.rpm \
   --output ./output \
   --verbose \
   --skip-cleanup \
   --patch ./0001-test.patch
```

For details about how to create a patch, see [syscare-build/README.md](https://gitee.com/openeuler/syscare/blob/master/syscare-build/README.md).

### Patch Management

1. Install the patch.

    ```bash
    $ sudo syscare apply redis-6.2.5-1/HP001
    ```

2. Activate the patch.

    ```bash
    $ sudo syscare active redis-6.2.5-1/HP001
    ```

3. Deactivate the patch.

    ```bash
    $ sudo syscare deactive redis-6.2.5-1/HP001
    ```

4. Uninstall or remove the patch.

    ```bash
    $ sudo syscare remove redis-6.2.5-1/HP001
    ```

5. Confirm the patch.

    ```bash
    $ sudo syscare accept redis-6.2.5-1/HP001
    ```

6. Check the patch status.

    ```bash
    $ syscare status redis-6.2.5-1/HP001
    ```

7. Query patch information.

    ```bash
    $ syscare info redis-6.2.5-1/HP001
    ```

8. Query information about the target patch software.

    ```bash
    $ syscare target redis-6.2.5-1/HP001
    ```

9. Query all patches.

    ```bash
    $ syscare list
    ```

### System Management

1. Quickly restart the system.

    ```bash
    $ syscare reboot
    ```

For details about how to use the command, see [cli/README.md](https://gitee.com/openeuler/syscare/blob/master/cli/README.md).

## Constraints

* Currently, only 64-bit systems are supported.
* Currently, only hot fixes in ELF format are supported. Interpreted languages are not supported.
* Currently, only the GCC or G++ compiler is supported.
* The compiler must support the `-gdwarf`, `-ffunction-sections`, and `-fdata-sections` parameters.
* Only debugging information in DWARF format is supported, and the g3 level is not supported.
* Global variables cannot be modified.
* Currently, cross compilation is not supported.
* Currently, assembly modification is not supported.
* Currently, external symbols (dynamic library dependency) cannot be added.
* Currently, multiple patches cannot be applied to the same binary file.
* Currently, in patch files with the same file name, symbols with **Bind** set to **Local** and **Type** set to `STT_FUNC` or `STT_OBJECT` cannot be identical.
  The local variables and function names of the files with the same name are completely consistent, but the implementation may differ.
* Currently, hybrid compilation of C and C++ is not supported.
* Currently, C++ exception modification is not supported.
* Currently, the group section: `-g3` compilation options, specific compilation optimization options, and specific GCC plugins are not supported.
* Currently, ifunc: `__attribute__((ifunc("foo")))` is not supported.
* Currently, the TLS variable `__thread int foo` is not supported.
* Currently, the LTO option cannot be enabled during compilation.

## Contributions

1. Fork this repository: `$ git clone https://gitee.com/openeuler/syscare.git`
2. Create your own branch: `$ cd syscare & git branch -b Feature_XXXX`
3. Improve feature code: `$ vim src/upatch/xxxx & git commit -m ""`
4. Commit code: `$ git push origin`
5. Create a pull request (PR).

## Discussions

* You can add the WeChat ID **syscare** and apply to join the SysCare discussion group.
* You can post or reply to posts on the openEuler forum at [https://forum.openeuler.org/](https://forum.openeuler.org/).
