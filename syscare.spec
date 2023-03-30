%global debug_package %{nil}

%define kernel_devel_rpm %(echo $(rpm -q kernel-devel | head -n 1))
%define kernel_name %(echo $(rpm -q --qf "\%%{VERSION}-\%%{RELEASE}.\%%{ARCH}" %{kernel_devel_rpm}))
%define kernel_version %(echo $(rpm -q --qf "\%%{VERSION}-\%%{RELEASE}" %{kernel_devel_rpm}))

Name:           syscare
Version:        1.0.1
Release:        3
Summary:        system hot-fix service

License:        MulanPSL-2.0 and GPL-2.0-only
URL:            https://gitee.com/openeuler/syscare
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust cargo gcc gcc-g++ cmake make
BuildRequires:  elfutils-libelf-devel

Requires:       coreutils systemd kpatch-runtime
%ifarch x86_64
Requires:       %{name}-kmod >= 1.0.1-1
%endif

%description
SysCare is a system-level hot-fix software that provides single-machine-level and cluster-level security patches and system error hot-fixes for the operating system.
The host can fix the system problem without rebooting.

%ifarch x86_64
%package kmod
Summary:       Syscare kernel modules.
Requires:      kernel = %{kernel_version}
BuildRequires: kernel-devel
BuildRequires: make gcc bison flex

%description kmod
Syscare kernel modules dependency.
%endif

%package build
Summary:  Tools for build syscare patch.
Requires: %{name} = %{version}-%{release}
%ifarch x86_64
Requires: %{name}-kmod >= 1.0.1-1
%endif
Requires: kpatch make gcc openssl-devel dwarves python3-devel bison flex
Requires: elfutils-libelf-devel
Requires: rpm-build

%description build
Syscare build tools.

%prep
%autosetup -p1

%build
mkdir -p build_tmp
cd build_tmp

cmake -DCMAKE_INSTALL_PREFIX=/usr -DBUILD_VERSION=%{version}-%{release} -DKERNEL_VERSION=%{kernel_name} ..
make

%install
cd build_tmp
%make_install
%ifarch x86_64
mkdir -p %{buildroot}/lib/modules/%{kernel_name}/extra/syscare
mv %{buildroot}/usr/libexec/syscare/upatch.ko %{buildroot}/lib/modules/%{kernel_name}/extra/syscare
%endif

%post
# Create runtime directory at installation
if [ "$1" -eq 1 ]; then
    mkdir -p /usr/lib/syscare/patches
fi

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

%ifarch x86_64
%post kmod
# Generate kernel module list
depmod -a > /dev/null 2>&1 || true

# Start all services
systemctl enable syscare-upatch
systemctl start syscare-upatch

%preun kmod
# Stop all services
systemctl stop syscare-upatch
systemctl disable syscare-upatch

%postun kmod
# Generate kernel module list
depmod -a > /dev/null 2>&1 || true
%endif

%files
%defattr(-,root,root,-)
%attr(755,root,root) /usr/bin/syscare
%attr(644,root,root) /usr/lib/systemd/system/syscare.service
%ifarch x86_64
%dir /usr/libexec/syscare
%attr(755,root,root) /usr/libexec/syscare/upatch-tool
%endif

%ifarch x86_64
%files kmod
%dir /lib/modules/%{kernel_name}/extra/syscare
%attr(640,root,root) /lib/modules/%{kernel_name}/extra/syscare/upatch.ko
%attr(644,root,root) /usr/lib/systemd/system/syscare-upatch.service
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
