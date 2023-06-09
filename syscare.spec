%global debug_package %{nil}

%define kernel_devel_rpm %(echo $(rpm -q kernel-devel | head -n 1))
%define kernel_version %(echo $(rpm -q --qf "\%%{VERSION}" %{kernel_devel_rpm}))
%define kernel_name %(echo $(rpm -q --qf "\%%{VERSION}-\%%{RELEASE}.\%%{ARCH}" %{kernel_devel_rpm}))

Name:           syscare
Version:        1.0.1
Release:        9
Summary:        system hot-fix service

License:        MulanPSL-2.0 and GPL-2.0-only
URL:            https://gitee.com/openeuler/syscare
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust cargo gcc gcc-g++ cmake make
BuildRequires:  elfutils-libelf-devel

Requires:       coreutils systemd kpatch-runtime
Requires:       %{name}-kmod >= 1.0.1-1

%description
SysCare is a system-level hot-fix software that provides single-machine-level and cluster-level security patches and system error hot-fixes for the operating system.
The host can fix the system problem without rebooting.

%package kmod
Summary:       Syscare kernel modules.
Requires:      kernel >= %{kernel_version}
BuildRequires: kernel-devel
BuildRequires: make gcc bison flex

%description kmod
Syscare kernel modules dependency.

%package build
Summary:  Tools for build syscare patch.
Requires: %{name} = %{version}-%{release}
Requires: %{name}-kmod >= 1.0.1-1
Requires: make gcc patch
Requires: bison flex
Requires: kpatch dwarves
Requires: elfutils-libelf-devel
Requires: rpm-build tar gzip

%description build
Syscare build tools.

%prep
%autosetup -p1

%build
mkdir -p build
cd build

cmake -DCMAKE_INSTALL_PREFIX=/usr -DBUILD_VERSION=%{version}-%{release} -DKERNEL_VERSION=%{kernel_name} ..
make

%install
cd build
%make_install
mkdir -p %{buildroot}/lib/modules/%{kernel_name}/extra/syscare
mv %{buildroot}/usr/libexec/syscare/upatch.ko %{buildroot}/lib/modules/%{kernel_name}/extra/syscare

%post
# Create runtime directory
mkdir -p /usr/lib/syscare/patches

# Start all services
systemctl enable syscare
systemctl start syscare

%preun
# Stop all services
systemctl stop syscare
systemctl disable syscare

%postun
# Remove runtime directory at uninstallation
if [ "$1" -eq 0 ] || { [ -n "$2" ] && [ "$2" -eq 0 ]; }; then
    rm -rf /usr/lib/syscare
fi

%post kmod
# Create kmod weak-updates link
echo "/lib/modules/%{kernel_name}/extra/syscare/upatch.ko" | /sbin/weak-modules --add-module --no-initramfs --verbose >&2

# Start all services
systemctl enable syscare-upatch
systemctl start syscare-upatch

%preun kmod
# Stop all services
systemctl stop syscare-upatch
systemctl disable syscare-upatch

%postun kmod
# Remove kmod weak-updates link
echo "/lib/modules/%{kernel_name}/extra/syscare/upatch.ko" | /sbin/weak-modules --remove-module --no-initramfs --verbose >&2

%files
%defattr(-,root,root,-)
%attr(755,root,root) /usr/bin/syscare
%attr(644,root,root) /usr/lib/systemd/system/syscare.service
%dir /usr/libexec/syscare
%attr(755,root,root) /usr/libexec/syscare/upatch-tool

%files kmod
%dir /lib/modules/%{kernel_name}/extra/syscare
%attr(640,root,root) /lib/modules/%{kernel_name}/extra/syscare/upatch.ko
%attr(644,root,root) /usr/lib/systemd/system/syscare-upatch.service

%files build
%defattr(-,root,root,-)
%dir /usr/libexec/syscare
%attr(755,root,root) /usr/libexec/syscare/syscare-build
%attr(755,root,root) /usr/libexec/syscare/upatch-build
%attr(755,root,root) /usr/libexec/syscare/upatch-diff

%changelog
* Fri Jun 09 2023 renoseven<dev@renoseven.net> - 1.0.1-9
- Fix 'patch file is not checked' issue
- Rename patched source package
- Update dependencies
* Tue Jun 06 2023 renoseven<dev@renoseven.net> - 1.0.1-8
- Fix 'kernel patch sys interface collision' issue
- Fix 'patch GOT table jump fails' issue
- Fix 'patch TLS variable relocation fails' issue
* Fri Jun 02 2023 renoseven<dev@renoseven.net> - 1.0.1-7
- Various bugfix
- Support multiple compiler
* Wed May 31 2023 renoseven<dev@renoseven.net> - 1.0.1-6
- Various bugfix
- Support multiple debuginfo package
* Mon May 15 2023 renoseven<dev@renoseven.net> - 1.0.1-5
- Fix aarch64 kmod patch jump instruction error issue
- Add ifunc support
- Add 'syscare accept' command
- Add patch 'ACCEPT' state
* Tue Apr 04 2023 renoseven<dev@renoseven.net> - 1.0.1-4
- Enable aarch64
- Fix syscare-upatch service may start failed issue
* Thu Mar 30 2023 renoseven<dev@renoseven.net> - 1.0.1-3
- Fix upatch may not contain all symbols issue
- Add syscare-kmod package
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
