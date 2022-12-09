%global debug_package %{nil}

Name:           syscare
Version:        0.1.2.rc3
Release:        1
Summary:        system hot-fix service

License:        MulanPSL-2.0 GPL-2.0-only
URL:            https://gitee.com/openeuler/syscare
Source0:        %{name}-%{version}.tar.gz

ExclusiveArch:  x86_64

BuildRequires:  rust cargo gcc gcc-g++ cmake make
BuildRequires:  elfutils-libelf-devel
BuildRequires:  kernel-devel

Requires:       kpatch-runtime

%description
SysCare is a system-level hot-fix software that provides single-machine-level and cluster-level security patches and system error hot-fixes for the operating system.
The host can fix the system problem without rebooting.

%package build
Summary:        Tools for build syscare patch.
Requires:       %{name} = %{version}-%{release}
Requires:       kpatch make gcc openssl-devel dwarves python3-devel bison flex
Requires:       rpm-build

%description build
Syscare build tools.

%define kernel_version $(rpm -q --qf "\%%{VERSION}-\%%{RELEASE}.\%%{ARCH}" `rpm -q kernel-devel` | head -n 1)

%prep
%autosetup -p1

%build
mkdir -p tmp_build
cd tmp_build
cmake -DSYSCARE_BUILD_VERSION=%{version} -DKERNEL_VERSION=%{kernel_version} ..
make

%install
cd tmp_build
%make_install

mkdir -p %{buildroot}/lib/modules/%{kernel_version}/extra/syscare
install -m 0640 %{buildroot}/usr/libexec/%{name}/upatch.ko %{buildroot}/lib/modules/%{kernel_version}/extra/syscare

mkdir -p %{buildroot}/usr/lib/systemd/system
install -m 0644 %{_builddir}/%{name}-%{version}/misc/%{name}-restore.service %{buildroot}/usr/lib/systemd/system
install -m 0644 %{_builddir}/%{name}-%{version}/misc/%{name}-pre.service %{buildroot}/usr/lib/systemd/system

mkdir -p %{buildroot}/usr/lib/syscare

cd %{buildroot}
find lib -name "upatch.ko" \
	-fprintf %{_builddir}/%{name}-%{version}/ko.files.list "/%p\n"

%post
%systemd_post %{name}-restore.service
%{_bindir}/systemctl enable %{name}-pre.service
depmod -A

%preun
%systemd_preun %{name}-restore.service
%systemd_preun %{name}-pre.service

%postun
depmod -A

%files -f ko.files.list
%files
%defattr(-,root,root,-)
%dir /usr/lib/syscare
%attr(755,root,root) /usr/bin/syscare
%attr(755,root,root) /usr/libexec/%{name}/upatch-tool
%attr(755,root,root) /usr/libexec/%{name}/auto-recovery.sh
%attr(640,root,root) /usr/libexec/%{name}/upatch.ko
%attr(644,root,root) /usr/lib/systemd/system/%{name}-restore.service
%attr(644,root,root) /usr/lib/systemd/system/%{name}-pre.service

%files build
%defattr(-,root,root,-)
%dir /usr/libexec/%{name}
%attr(755,root,root) /usr/libexec/%{name}/upatch-diff
%attr(755,root,root) /usr/libexec/%{name}/upatch-build
%attr(755,root,root) /usr/libexec/%{name}/syscare-build

%changelog
* Fri Dec 9 2022 snoweay<snoweay@163.com> - 0.1.2.rc3-1
- Enable upatch.ko compile. Fix some recovery issues.
* Thu Dec 8 2022 snoweay<snoweay@163.com> - 0.1.2.rc2-1
- 0.1.2.rc2 version for test. Fix some issues..
* Thu Dec 1 2022 snoweay<snoweay@163.com> - 0.1.2-1
- 0.1.2 version for test. Fix some syscare cli bugs.
* Mon Nov 28 2022 snoweay<snoweay@163.com> - 0.1.1-1
- First version for test. Support patches restore, remove, insmod upatch.ko.
* Mon Nov 21 2022 snoweay<snoweay@163.com> - 0.1.0-1
- init version for 0.1.1-1.
