%global debug_package %{nil}

Name:           syscare
Version:        1.0.0
Release:        5
Summary:        system hot-fix service

License:        MulanPSL-2.0 GPL-2.0-only
URL:            https://gitee.com/openeuler/syscare
Source0:        %{name}-%{version}.tar.gz
Patch1:         v1.0.0-5.patch

BuildRequires:  rust cargo gcc gcc-g++ cmake make
BuildRequires:  elfutils-libelf-devel
BuildRequires:  kernel-devel

Requires:       kpatch-runtime coreutils

%description
SysCare is a system-level hot-fix software that provides single-machine-level and cluster-level security patches and system error hot-fixes for the operating system.
The host can fix the system problem without rebooting.

%package build
Summary:        Tools for build syscare patch.
Requires:       %{name} = %{version}-%{release}
Requires:       kpatch make gcc openssl-devel dwarves python3-devel bison flex
Requires:       elfutils-libelf-devel
Requires:       rpm-build

%description build
Syscare build tools.

%define kernel_version $(rpm -q --qf "\%%{VERSION}-\%%{RELEASE}.\%%{ARCH}" `rpm -q kernel-devel` | head -n 1)

%prep
%autosetup -p1

%build
mkdir -p tmp_build
cd tmp_build
cmake -DSYSCARE_BUILD_VERSION=%{version}-%{release} -DKERNEL_VERSION=%{kernel_version} ..
make

%install
cd tmp_build
%make_install

mkdir -p %{buildroot}/lib/modules/%{kernel_version}/extra/syscare
%ifarch x86_64
install -m 0640 %{buildroot}/usr/libexec/%{name}/upatch.ko %{buildroot}/lib/modules/%{kernel_version}/extra/syscare
%endif

mkdir -p %{buildroot}/usr/lib/systemd/system
%ifarch aarch64
install -m 0644 %{_builddir}/%{name}-%{version}/misc/%{name}-restore-arm64.service %{buildroot}/usr/lib/systemd/system/%{name}-restore.service
%else
install -m 0644 %{_builddir}/%{name}-%{version}/misc/%{name}-restore.service %{buildroot}/usr/lib/systemd/system
install -m 0644 %{_builddir}/%{name}-%{version}/misc/%{name}-pre.service %{buildroot}/usr/lib/systemd/system
%endif

mkdir -p %{buildroot}/usr/lib/syscare

%ifarch x86_64
cd %{buildroot}
find lib -name "upatch.ko" \
	-fprintf %{_builddir}/%{name}-%{version}/ko.files.list "/%p\n"
%endif

%post
%systemd_post %{name}-restore.service
%ifarch x86_64
%{_bindir}/systemctl enable %{name}-pre.service
%endif
depmod -a > /dev/null 2>&1 || true

%preun
%systemd_preun %{name}-restore.service
%ifarch x86_64
%systemd_preun %{name}-pre.service
%endif

%postun
depmod -a > /dev/null 2>&1 || true

%ifarch x86_64
%files -f ko.files.list
%endif
%files
%defattr(-,root,root,-)
%dir /usr/lib/syscare
%attr(755,root,root) /usr/bin/syscare
%ifarch x86_64
%attr(755,root,root) /usr/libexec/%{name}/upatch-tool
%attr(640,root,root) /usr/libexec/%{name}/upatch.ko
%attr(644,root,root) /usr/lib/systemd/system/%{name}-pre.service
%endif
%attr(755,root,root) /usr/libexec/%{name}/auto-recovery.sh
%attr(644,root,root) /usr/lib/systemd/system/%{name}-restore.service

%files build
%defattr(-,root,root,-)
%dir /usr/libexec/%{name}
%attr(755,root,root) /usr/libexec/%{name}/syscare-build
%ifarch x86_64
%attr(755,root,root) /usr/libexec/%{name}/upatch-diff
%attr(755,root,root) /usr/libexec/%{name}/upatch-build
%endif

%changelog
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
