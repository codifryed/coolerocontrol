# rpkg variant
%global _enable_debug_packages 0
%global debug_package %{nil}
%global project coolercontrol

Name:           %{project}-liqctld
Version:        0.17.2
Release:        1%{?dist}
Summary:        Monitor and control your cooling devices.

License:        GPLv3+
URL:            https://gitlab.com/%{project}/%{project}

BuildRequires:  systemd-rpm-macros
BuildRequires:  python3-devel
BuildRequires:  python3-wheel
BuildRequires:  python3-liquidctl
BuildRequires:  python3-setproctitle
BuildRequires:  python3-fastapi
BuildRequires:  python3-uvicorn

VCS:        {{{ git_dir_vcs }}}
Source:     {{{ git_dir_pack }}}

%description
CoolerControl is a program to monitor and control your cooling devices.

It offers an easy-to-use user interface with various control features and also provides live thermal performance details.

%prep
{{{ git_dir_setup_macro }}}

%generate_buildrequires
%pyproject_buildrequires

%build
%pyproject_wheel

%install
%pyproject_install
%pyproject_save_files coolercontrol_liqctld
mkdir -p %{buildroot}%{_unitdir}
cp -p coolercontrol-liqctld.service %{buildroot}%{_unitdir}

%check
%pyproject_check_import

%files -f %{pyproject_files}
%{_bindir}/coolercontrol-liqctld
%{_unitdir}/coolercontrol-liqctld.service
%license LICENSE
%doc README.md CHANGELOG.md

%changelog
* Tue Nov 28 2023 Guy Boldon <gb@guyboldon.com> - 0.17.2-0
- 0.17.2 Release

* Wed Sep 13 2023 Guy Boldon <gb@guyboldon.com> - 0.17.1-0
- 0.17.1 Release

* Sun Jul 16 2023 Guy Boldon <gb@guyboldon.com> - 0.17.0-0
- 0.17.0 Release

* Sun Apr 23 2023 Guy Boldon <gb@guyboldon.com> - 0.16.0-0
- 0.16.0 Release

* Tue Mar 14 2023 Guy Boldon <gb@guyboldon.com> - 0.15.0-0
- 0.15.0 Release

* Wed Mar 01 2023 Guy Boldon <gb@guyboldon.com> - 0.14.6-0
- 0.14.6 Release

* Mon Feb 27 2023 Guy Boldon <gb@guyboldon.com> - 0.14.5-0
- 0.14.5 Release

* Tue Feb 14 2023 Guy Boldon <gb@guyboldon.com> - 0.14.4-0
- 0.14.4 Release

* Thu Feb 09 2023 Guy Boldon <gb@guyboldon.com> - 0.14.3-0
- 0.14.3 Release

* Tue Feb 07 2023 Guy Boldon <gb@guyboldon.com> - 0.14.2-0
- 0.14.2 Release

* Mon Feb 06 2023 Guy Boldon <gb@guyboldon.com> - 0.14.1-0
- 0.14.1 Release

* Sun Feb 05 2023 Guy Boldon <gb@guyboldon.com> - 0.14.0-0
- 0.14.0 Release
