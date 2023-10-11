%define _debugsource_template %{nil}

%define build_version    %{version}-%{release}
%define kernel_devel_rpm %(echo $(rpm -q kernel-devel | head -n 1))
%define kernel_version   %(echo $(rpm -q --qf "\%%{VERSION}" %{kernel_devel_rpm}))
%define kernel_name      %(echo $(rpm -q --qf "\%%{VERSION}-\%%{RELEASE}.\%%{ARCH}" %{kernel_devel_rpm}))

############################################
############ Package syscare ###############
############################################
Name:          syscare
Version:       1.1.0
Release:       6k
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
    -D_BUILD_UPATCH=0 \
    ..

make

################# Install ##################
%install
cd build
%make_install

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

############################################
########## Package syscare-build ###########
############################################
%package build
Summary: Syscare build tools.
BuildRequires: elfutils-libelf-devel
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

############### PreUninstall ###############
%preun build

############## PostUninstall ###############
%postun build
if [ "$1" -eq 0 ] || { [ -n "$2" ] && [ "$2" -eq 0 ]; }; then
    if [ -z "$(ls -A /var/log/syscare)" ]; then
        rm -rf /var/log/syscare
    fi
fi

################## Files ###################
%files build
%defattr(-,root,root,-)
%dir /usr/libexec/syscare
%attr(755,root,root) /usr/libexec/syscare/syscare-build

############################################
################ Change log ################
############################################
%changelog
* Wed Oct 11 2023 renoseven<dev@renoseven.net> - 1.1.0-6k
- Disable upatch support
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
