%define build_version    %{version}-%{release}
%define kernel_devel_rpm %(echo $(rpm -q kernel-devel | head -n 1))
%define kernel_version   %(echo $(rpm -q --qf "\%%{VERSION}" %{kernel_devel_rpm}))
%define kernel_name      %(echo $(rpm -q --qf "\%%{VERSION}-\%%{RELEASE}.\%%{ARCH}" %{kernel_devel_rpm}))

%define pkg_kmod       %{name}-kmod
%define pkg_build      %{name}-build
%define pkg_build_kmod %{pkg_build}-kmod
%define pkg_build_ebpf %{pkg_build}-ebpf

############################################
############ Package syscare ###############
############################################
Name:          syscare
Version:       1.2.0
Release:       3
Summary:       System hot-fix service
License:       MulanPSL-2.0 and GPL-2.0-only
URL:           https://gitee.com/openeuler/syscare
Source0:       %{name}-%{version}.tar.gz
BuildRequires: cmake >= 3.14 make
BuildRequires: rust >= 1.51 cargo >= 1.51
BuildRequires: gcc gcc-c++
Requires:      coreutils systemd
Requires:      kpatch-runtime

############### Description ################
%description
SysCare is a system-level hot-fix service that provides security patches and system error hot-fixes for the operating system.
The host can fix the system problem without rebooting.

############## BuildPreparare ##############
%prep
%autosetup -p1

################## Build ###################
%build
mkdir -p build
cd build

cmake \
    -DCMAKE_INSTALL_PREFIX=/usr \
    -DBUILD_VERSION=%{build_version} \
    -DKERNEL_VERSION=%{kernel_name} \
    ..

make

################# Install ##################
%install
cd build
%make_install

mkdir -p %{buildroot}/lib/modules/%{kernel_name}/extra/syscare
mv -f %{buildroot}/usr/libexec/syscare/upatch_hijacker.ko %{buildroot}/lib/modules/%{kernel_name}/extra/syscare

############### PostInstall ################
%post
mkdir -p /usr/lib/syscare/patches

systemctl daemon-reload
systemctl enable syscare
systemctl start syscare

############### PreUninstall ###############
%preun
systemctl daemon-reload
systemctl stop syscare
systemctl disable syscare

############## PostUninstall ###############
%postun
if [ "$1" -eq 0 ] || { [ -n "$2" ] && [ "$2" -eq 0 ]; }; then
    rm -rf /usr/lib/syscare
    rm -f /var/log/syscare/syscared*.log*
    if [ -z "$(ls -A /var/log/syscare)" ]; then
        rm -rf /var/log/syscare
    fi
fi

################## Files ###################
%files
%defattr(-,root,root,-)
%dir /usr/libexec/syscare
%attr(644,root,root) /usr/lib/systemd/system/syscare.service
%attr(755,root,root) /usr/bin/syscared
%attr(755,root,root) /usr/bin/syscare
%attr(755,root,root) /usr/libexec/syscare/upatch-manage

############################################
########## Package syscare-build ###########
############################################
%package build
Summary: Syscare build tools.
BuildRequires: elfutils-libelf-devel
Requires: (%{pkg_build_kmod} >= %{build_version} or %{pkg_build_ebpf} >= %{build_version})
Requires: coreutils
Requires: patch
Requires: kpatch
Requires: tar gzip
Requires: rpm rpm-build

############### Description ################
%description build
Syscare patch building toolset.

############### PostInstall ################
%post build
mkdir -p /etc/syscare
systemctl daemon-reload
systemctl enable upatch
systemctl start upatch

############### PreUninstall ###############
%preun build
systemctl daemon-reload
systemctl stop upatch
systemctl disable upatch

############## PostUninstall ###############
%postun build
if [ "$1" -eq 0 ] || { [ -n "$2" ] && [ "$2" -eq 0 ]; }; then
    rm -rf /etc/syscare
    rm -f /var/log/syscare/upatchd*.log*
    if [ -z "$(ls -A /var/log/syscare)" ]; then
        rm -rf /var/log/syscare
    fi
fi

################## Files ###################
%files build
%defattr(-,root,root,-)
%dir /usr/libexec/syscare
%attr(644,root,root) /usr/lib/systemd/system/upatch.service
%attr(755,root,root) /usr/bin/upatchd
%attr(755,root,root) /usr/libexec/syscare/syscare-build
%attr(755,root,root) /usr/libexec/syscare/upatch-build
%attr(755,root,root) /usr/libexec/syscare/upatch-diff
%attr(755,root,root) /usr/libexec/syscare/as-hijacker
%attr(755,root,root) /usr/libexec/syscare/cc-hijacker
%attr(755,root,root) /usr/libexec/syscare/c++-hijacker
%attr(755,root,root) /usr/libexec/syscare/gcc-hijacker
%attr(755,root,root) /usr/libexec/syscare/g++-hijacker

############################################
######## Package syscare-build-kmod ########
############################################
%package build-kmod
Summary: Kernel module for syscare patch build tools.
BuildRequires: make gcc
BuildRequires: kernel-devel
Requires: kernel >= %{kernel_version}
Conflicts: %{pkg_build_ebpf}

############### Description ################
%description build-kmod
Syscare build dependency - kernel module.

############### PostInstall ################
%post build-kmod
echo "/lib/modules/%{kernel_name}/extra/syscare/upatch_hijacker.ko" | /sbin/weak-modules --add-module --no-initramfs
depmod

############### PreUninstall ###############
%preun build-kmod
# Nothing

############## PostUninstall ###############
%postun build-kmod
echo "/lib/modules/%{kernel_name}/extra/syscare/upatch_hijacker.ko" | /sbin/weak-modules --remove-module --no-initramfs
depmod

################## Files ###################
%files build-kmod
%dir /lib/modules/%{kernel_name}/extra/syscare
%attr(640,root,root) /lib/modules/%{kernel_name}/extra/syscare/upatch_hijacker.ko

############################################
######## Package syscare-build-ebpf ########
############################################
%package build-ebpf
Summary: eBPF for syscare patch build tools.
BuildRequires: make llvm clang bpftool
BuildRequires: libbpf libbpf-devel libbpf-static
Conflicts: %{pkg_build_kmod}

############### Description ################
%description build-ebpf
Syscare build dependency - eBPF.

############### PostInstall ################
%post build-ebpf

############### PreUninstall ###############
%preun build-ebpf
# Nothing

############## PostUninstall ###############
%postun build-ebpf
# Nothing

################## Files ###################
%files build-ebpf
%attr(755,root,root) /usr/libexec/syscare/upatch_hijacker

############################################
################ Change log ################
############################################
%changelog
* Fri Nov 24 2023 renoseven<dev@renoseven.net> - 1.2.0-3
- Fix 'upatch only apply first patch for new process' issue
* Wed Nov 22 2023 renoseven<dev@renoseven.net> - 1.2.0-2
- Fix upatch process detection
* Wed Nov 22 2023 renoseven<dev@renoseven.net> - 1.2.0-1
- Fix various issue
* Wed Oct 11 2023 renoseven<dev@renoseven.net> - 1.1.0-6
- Support build patch for kernel moudules
- Fix various issue
* Fri Sep 22 2023 renoseven<dev@renoseven.net> - 1.1.0-5
- Fix various issue
* Thu Sep 21 2023 renoseven<dev@renoseven.net> - 1.1.0-4
- Fix 'syscare-build only accept one patch' issue
* Wed Sep 20 2023 renoseven<dev@renoseven.net> - 1.1.0-3
- Fix various issue
- Support MSRV 1.51
* Mon Aug 28 2023 renoseven<dev@renoseven.net> - 1.1.0-1
- Support build patch without kernel module
- Add syscare daemon
- Add syscare-build daemon
- Improve syscare cli
* Wed Jun 28 2023 renoseven<dev@renoseven.net> - 1.0.2-4
- Fix builder check failure issue
* Sun Jun 25 2023 renoseven<dev@renoseven.net> - 1.0.2-3
- Fix various issue
* Mon Jun 19 2023 renoseven<dev@renoseven.net> - 1.0.2-2
- Fix various issue
- Update dependencies
* Fri Jun 09 2023 renoseven<dev@renoseven.net> - 1.0.2-1
- Fix 'rpmpbuild getcwd failed' issue
- Fix 'upatch ko prints redundant log' issue
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
