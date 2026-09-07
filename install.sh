#!/usr/bin/env bash
set -e

if [ "$EUID" -ne 0 ]; then
  echo "Please run as root using: sudo ./install.sh"
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="${SCRIPT_DIR}/build"

echo "==> Installing updated resources binary..."
cp "${BUILD_DIR}/src/release/resources" /usr/bin/resources
chmod 755 /usr/bin/resources

echo "==> Installing npu-data-exporter binary..."
cp "${BUILD_DIR}/src/release/npu-data-exporter" /usr/local/bin/npu-data-exporter
chmod 755 /usr/local/bin/npu-data-exporter

echo "==> Installing compiled GResource..."
mkdir -p /usr/share/resources
cp "${BUILD_DIR}/data/resources/resources.gresource" /usr/share/resources/resources.gresource
chmod 644 /usr/share/resources/resources.gresource

echo "==> Installing GSchema..."
cp "${SCRIPT_DIR}/data/org.gnome.Resources.gschema.xml.in" /usr/share/glib-2.0/schemas/org.gnome.Resources.gschema.xml
chmod 644 /usr/share/glib-2.0/schemas/org.gnome.Resources.gschema.xml
glib-compile-schemas /usr/share/glib-2.0/schemas/

echo "==> Installing npu-data-exporter systemd service..."
cat <<'SERVICE_EOF' > /etc/systemd/system/npu-data-exporter.service
[Unit]
Description=AMD XDNA NPU Data Exporter Daemon
After=syslog.target network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/npu-data-exporter
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
SERVICE_EOF

systemctl daemon-reload
systemctl enable --now npu-data-exporter.service || true

echo "==> Installation complete! GNOME Resources and npu-data-exporter service are now active."
