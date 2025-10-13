Name:           process-exporter
Version:        %{_version}
Release:        1%{?dist}
Summary:        Process Exporter with eBPF Network Monitoring

License:        MIT
URL:            https://github.com/cylonchau/process-exporter-rust

# Pre-built files
Source0:        process-exporter
Source1:        process-exporter.service
Source2:        process-exporter.env
Source3:        install-deps.sh
Source4:        README.md
Source5:        LICENSE

BuildArch:      x86_64

Requires:       systemd
Requires:       glibc
Requires:       elfutils-libelf
Recommends:     kernel-devel
Recommends:     kernel-headers

%description
A Prometheus exporter for dynamic process monitoring with rust (rust pratices project).
with eBPF-based network traffic tracking.

Features:
- Dynamic process registration via REST API
- eBPF-based network monitoring for per-process network statistics
- Support for CPU, memory, disk I/O, and network monitoring

Note: eBPF functionality requires kernel headers. Run the following
command after installation to install required dependencies:
  sudo /usr/share/process-exporter/install-deps.sh --runtime

%prep
# Copy documentation to BUILD directory for %doc and %license macros
cp %{SOURCE4} .
cp %{SOURCE5} .

%build
# No build needed - using pre-built binary

%install
# Create directories
install -d %{buildroot}%{_sysconfdir}/%{name}
install -d %{buildroot}%{_localstatedir}/log/%{name}

# Install binary
install -D -m 755 %{SOURCE0} %{buildroot}%{_bindir}/%{name}

# Install systemd service
install -D -m 644 %{SOURCE1} %{buildroot}%{_unitdir}/%{name}.service

# Install environment configuration
install -D -m 644 %{SOURCE2} %{buildroot}%{_sysconfdir}/%{name}/%{name}.env

# Install dependency script
install -D -m 755 %{SOURCE3} %{buildroot}%{_datadir}/%{name}/install-deps.sh

%post
%systemd_post %{name}.service

# Create log directory
mkdir -p %{_localstatedir}/log/%{name}
chown root:root %{_localstatedir}/log/%{name}
chmod 755 %{_localstatedir}/log/%{name}

# Set config file permissions
chown root:root %{_sysconfdir}/%{name}/%{name}.env
chmod 644 %{_sysconfdir}/%{name}/%{name}.env

cat <<EOF

╔═══════════════════════════════════════════════════════════╗
║  Process Exporter installed successfully!                 ║
╚═══════════════════════════════════════════════════════════╝

⚠️  IMPORTANT: Install runtime dependencies first!

Run the following command to install required dependencies:
  sudo /usr/share/%{name}/install-deps.sh --runtime

Configuration file:
  %{_sysconfdir}/%{name}/%{name}.env

Edit configuration if needed:
  sudo vi %{_sysconfdir}/%{name}/%{name}.env

After installing dependencies, start the service:
  sudo systemctl enable --now %{name}

View logs:
  sudo journalctl -u %{name} -f

Access metrics:
  curl http://localhost:9999/metrics

EOF

%preun
%systemd_preun %{name}.service

%postun
%systemd_postun_with_restart %{name}.service

# Clean up on complete removal
if [ $1 -eq 0 ]; then
    rm -rf %{_localstatedir}/log/%{name}
fi

%files
%license LICENSE
%doc README.md
%{_bindir}/%{name}
%{_unitdir}/%{name}.service
%dir %{_sysconfdir}/%{name}
%config(noreplace) %{_sysconfdir}/%{name}/%{name}.env
%dir %{_datadir}/%{name}
%{_datadir}/%{name}/install-deps.sh
%dir %{_localstatedir}/log/%{name}

%changelog
* Mon Oct 13 2025 cylonchau <cylonchau@outlook.com> - 0.1.1-1
- Initial RPM release
- eBPF-based network monitoring
- Dynamic process registration API
- Prometheus metrics export