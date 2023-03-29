%global debug_package %{nil}

Name:           syscare
Version:        1.0.1
Release:        2
Summary:        system hot-fix service

License:        MulanPSL-2.0 and GPL-2.0-only
URL:            https://gitee.com/openeuler/syscare
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust cargo gcc gcc-g++ cmake make
BuildRequires:  elfutils-libelf-devel
BuildRequires:  kernel-devel

Requires:       coreutils systemd kpatch-runtime

%description
SysCare is a system-level hot-fix software that provides single-machine-level and cluster-level security patches and system error hot-fixes for the operating system.
The host can fix the system problem without rebooting.

%package build
Summary:  Tools for build syscare patch.
Requires: %{name} = %{version}-%{release}
Requires: kpatch make gcc openssl-devel dwarves python3-devel bison flex
Requires: elfutils-libelf-devel
Requires: rpm-build

%description build
Syscare build tools.

%define kernel_version $(rpm -q --qf "\%%{VERSION}-\%%{RELEASE}.\%%{ARCH}" `rpm -q kernel-devel` | head -n 1)

%prep
%autosetup -p1

%build
mkdir -p build_tmp
cd build_tmp

cmake -DCMAKE_INSTALL_PREFIX=/usr -DBUILD_VERSION=%{version}-%{release} -DKERNEL_VERSION=%{kernel_version} ..
make

%install
cd build_tmp
%make_install

%post
# Create runtime directory at installation
if [ "$1" -eq 1 ]; then
    mkdir -p /usr/lib/syscare/patches
fi

%ifarch x86_64
# Copy upatch kernel module to lib/modules
mkdir -p /lib/modules/$(uname -r)/extra/syscare
install -m 0644 /usr/libexec/syscare/upatch.ko /lib/modules/$(uname -r)/extra/syscare

# Generate kernel module list
depmod -a > /dev/null 2>&1 || true

# Start all services
systemctl enable syscare-pre.service
systemctl enable syscare.service
systemctl start syscare-pre
systemctl start syscare
%endif

%preun
%ifarch x86_64
# Stop all services
systemctl stop syscare.service
systemctl stop syscare-pre.service
systemctl disable syscare.service
systemctl disable syscare-pre.service

# Unload upatch kernel module
rmmod upatch > /dev/null 2>&1 || true
%endif

%postun
# Remove runtime directory at uninstallation
if [ "$1" -eq 0 ] || { [ -n "$2" ] && [ "$2" -eq 0 ]; }; then
    rm -rf /usr/lib/syscare
fi

%ifarch x86_64
# Remove upatch kernel module from lib/modules
rm -rf /lib/modules/$(uname -r)/extra/syscare

# Generate kernel module list
depmod -a > /dev/null 2>&1 || true
%endif

%files
%defattr(-,root,root,-)
%attr(755,root,root) /usr/bin/syscare
%attr(644,root,root) /usr/lib/systemd/system/syscare-pre.service
%attr(644,root,root) /usr/lib/systemd/system/syscare.service
%ifarch x86_64
%dir /usr/libexec/syscare
%attr(640,root,root) /usr/libexec/syscare/upatch.ko
%attr(755,root,root) /usr/libexec/syscare/upatch-tool
%endif

%files build
%defattr(-,root,root,-)
%dir /usr/libexec/syscare
%attr(755,root,root) /usr/libexec/syscare/syscare-build
%attr(755,root,root) /usr/libexec/syscare/upatch-build
%ifarch x86_64
%attr(755,root,root) /usr/libexec/syscare/upatch-diff
%endif

%changelog
* Wed Mar 29 2023 renoseven<dev@renoseven.net> - 1.0.1-2
- Fix rpm install & remove script issue
* Wed Mar 15 2023 renoseven<dev@renoseven.net> - 1.0.1-1
- New syscare cli
- Support building patch for C++ code
- Support patch version verification
- Support elf name derivation
- Support fast reboot
* Wed Dec 21 2022 snoweay<snoweay@163.com> - 1.0.0-7
- Fix 42 relocation caused by gcc 11.
* Tue Dec 20 2022 snoweay<snoweay@163.com> - 1.0.0-6
- Fix patch open failure by reading patches at attach instead of load.
- Support epoch in spec.
* Sat Dec 17 2022 snoweay<snoweay@163.com> - 1.0.0-5
- Check version-release of source pkg & debuginfo pkg.
* Fri Dec 16 2022 snoweay<snoweay@163.com> - 1.0.0-4
- Avoid duplicate elfs by not following symlinks at build.
* Thu Dec 15 2022 snoweay<snoweay@163.com> - 1.0.0-3
- Change kernel patches' scontext before apply not at rpm-post.
* Wed Dec 14 2022 snoweay<snoweay@163.com> - 1.0.0-2
- Fix some issues:
- manager: Allow apply to actived kernel patch
- build: only 'NOT-APPLIED' patch package can be removed
- build: fix 'kernel patch cannot be insmod during system start' issue
- kmod: unregister when rmmod upatch
* Tue Dec 13 2022 snoweay<snoweay@163.com> - 1.0.0-1
- Release the first version 1.0.0.
