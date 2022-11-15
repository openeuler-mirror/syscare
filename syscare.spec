%global debug_package %{nil}

Name:           syscare
Version:        0.1.1
Release:        1
Summary:        system hot-fix service

License:        MulanPSL-2.0, GPLv2
URL:            https://gitee.com/openeuler/syscare
Source0:        %{name}-%{version}.tar.gz

ExclusiveArch:  x86_64

BuildRequires:  rust cargo gcc gcc-g++ cmake make
BuildRequires:  elfutils-libelf-devel

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

%prep
%autosetup -p1

%build
cmake .
make

%install
%make_install

%files
%defattr(-,root,root,-)
%dir /usr/libexec/%{name}
%attr(750,root,root) /usr/bin/syscare

%files build
%defattr(-,root,root,-)
%dir /usr/libexec/%{name}
%attr(750,root,root) /usr/libexec/%{name}/upatch-diff
%attr(750,root,root) /usr/libexec/%{name}/upatch-build
%attr(750,root,root) /usr/libexec/%{name}/upatch-tool
%attr(750,root,root) /usr/libexec/%{name}/syscare-build

%changelog
* Mon Nov 21 2022 snoweay<snoweay@163.com> - 0.1.1-1
- init version for 0.1.1-1.
