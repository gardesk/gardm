Name:           gardm
Version:        0.1.0
Release:        1%{?dist}
Summary:        Display manager for the gar desktop suite

License:        MIT
URL:            https://github.com/gardesk/gardm
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust >= 1.75
BuildRequires:  cargo
BuildRequires:  pam-devel

Requires:       pam

# Disable debug package
%global debug_package %{nil}

%description
Gardm is a display manager for Linux built in Rust. Features a greeter UI,
PAM authentication, and systemd integration. Part of the gardesk desktop
environment suite.

%prep
%autosetup

%build
export CARGO_TARGET_DIR=target
cargo build --release --workspace

%install
install -Dm755 target/release/gardmd %{buildroot}%{_bindir}/gardmd
install -Dm755 target/release/gardm-greeter %{buildroot}%{_bindir}/gardm-greeter
install -Dm644 etc/gardm.service %{buildroot}%{_unitdir}/gardm.service
install -Dm644 etc/config.toml %{buildroot}%{_sysconfdir}/gardm/config.toml
install -Dm644 etc/pam.d/gardm %{buildroot}%{_sysconfdir}/pam.d/gardm

%files
%{_bindir}/gardmd
%{_bindir}/gardm-greeter
%{_unitdir}/gardm.service
%config(noreplace) %{_sysconfdir}/gardm/config.toml
%config(noreplace) %{_sysconfdir}/pam.d/gardm

%post
%systemd_post gardm.service

%preun
%systemd_preun gardm.service

%postun
%systemd_postun_with_restart gardm.service

%changelog
* Fri Jan 17 2025 mfw <espadonne@outlook.com> - 0.1.0-1
- Initial RPM release of gardm
- Display manager with greeter and PAM auth
- Part of gardesk desktop suite
