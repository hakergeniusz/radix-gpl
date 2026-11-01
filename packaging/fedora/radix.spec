# Spec for building Radix RPM packages. The version fields below track
# the git tag (v51.1.0 -> Version 51.1.0); bump them together with
# Cargo.toml, debian/changelog, the metainfo releases and the Arch
# PKGBUILD on each release.
# m4 is required by gmp-mpfr-sys, which builds its bundled GMP/MPFR/MPC
# from source at compile time (statically linked; no runtime gmp deps).
Name:           radix
Version:        51.1.0
Release:        1%{?dist}
Summary:        Desktop calculator with scientific, financial and programming modes

License:        MIT
URL:            https://github.com/hakergeniusz/radix
Source0:        %{url}/archive/refs/tags/v51.1.0/radix-51.1.0.tar.gz

BuildRequires:  cargo
BuildRequires:  gcc
BuildRequires:  make
BuildRequires:  m4
BuildRequires:  pkgconfig(gtk4) >= 4.12
BuildRequires:  pkgconfig(libadwaita-1) >= 1.5
# No -debuginfo/-debugsource subpackages: the release pipeline ships the
# main RPM only.
%define debug_package %{nil}

%global _description %{expand:
Radix is a clean-room Rust reimplementation of GNOME Calculator's behavior
(target: upstream v51.1 semantics). It ships one GTK4 GUI covering six
modes (basic, advanced, financial, programming, keyboard input and unit
conversion), the radix-cmd REPL, a headless `radix --solve` CLI and a
GNOME Shell search provider that answers straight from the search field.}

%description %{_description}

%prep
%autosetup -n radix-51.1.0

%build
cargo build --release --locked

%install
install -Dm755 target/release/radix %{buildroot}%{_bindir}/radix
install -Dm755 target/release/radix-cmd %{buildroot}%{_bindir}/radix-cmd
install -Dm755 target/release/radix-search-provider %{buildroot}%{_bindir}/radix-search-provider
install -Dm644 crates/radix-gui/data/applications/org.gnome.Radix.desktop \
  %{buildroot}%{_datadir}/applications/org.gnome.Radix.desktop
install -d %{buildroot}%{_datadir}/dbus-1/services
sed 's|@bindir@|%{_bindir}|' \
  crates/radix-gui/data/dbus-1/services/org.gnome.Radix.SearchProvider.service \
  > %{buildroot}%{_datadir}/dbus-1/services/org.gnome.Radix.SearchProvider.service
install -Dm644 crates/radix-gui/data/metainfo/org.gnome.Radix.metainfo.xml \
  %{buildroot}%{_datadir}/metainfo/org.gnome.Radix.metainfo.xml
install -Dm644 crates/radix-gui/data/icons/hicolor/scalable/apps/org.gnome.Radix.svg \
  %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/org.gnome.Radix.svg
install -Dm644 crates/radix-gui/data/icons/hicolor/symbolic/apps/org.gnome.Radix-symbolic.svg \
  %{buildroot}%{_datadir}/icons/hicolor/symbolic/apps/org.gnome.Radix-symbolic.svg

%check
./target/release/radix --version
./target/release/radix-cmd "2+2" | grep -qx 4

%files
%license LICENSE
%doc README.md
%{_bindir}/radix
%{_bindir}/radix-cmd
%{_bindir}/radix-search-provider
%{_datadir}/applications/org.gnome.Radix.desktop
%{_datadir}/dbus-1/services/org.gnome.Radix.SearchProvider.service
%{_datadir}/metainfo/org.gnome.Radix.metainfo.xml
%{_datadir}/icons/hicolor/scalable/apps/org.gnome.Radix.svg
%{_datadir}/icons/hicolor/symbolic/apps/org.gnome.Radix-symbolic.svg

%changelog
* Sat Nov 01 2026 hakergeniusz <hakergeniusz@users.noreply.github.com> - 51.1.0-1
- Initial packaged release: engine, six-mode GTK4 GUI, radix-cmd REPL,
  headless --solve CLI and the GNOME Shell search provider
