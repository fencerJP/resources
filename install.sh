#!/usr/bin/env bash
set -e

if [ "$EUID" -ne 0 ]; then
  echo "Please run as root using: sudo ./install.sh"
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="${SCRIPT_DIR}/build"

echo "==> Stopping any running npu-data-exporter or resources processes..."
systemctl stop npu-data-exporter.service 2>/dev/null || true
pkill -9 npu-data-exporter 2>/dev/null || true
pkill -9 resources 2>/dev/null || true
sleep 1

echo "==> Installing updated resources binary..."
cp -f "${BUILD_DIR}/src/release/resources" /usr/bin/resources
chmod 755 /usr/bin/resources

echo "==> Installing npu-data-exporter binary..."
rm -f /usr/local/bin/npu-data-exporter
cp -f "${BUILD_DIR}/src/release/npu-data-exporter" /usr/local/bin/npu-data-exporter
chmod 755 /usr/local/bin/npu-data-exporter

echo "==> Installing compiled GResource..."
mkdir -p /usr/share/resources
cp -f "${BUILD_DIR}/data/resources/resources.gresource" /usr/share/resources/resources.gresource
chmod 644 /usr/share/resources/resources.gresource

echo "==> Installing compiled GSchemas..."
cp -f "${BUILD_DIR}/data/org.gnome.Resources.gschema.xml" /usr/share/glib-2.0/schemas/org.gnome.Resources.gschema.xml
chmod 644 /usr/share/glib-2.0/schemas/org.gnome.Resources.gschema.xml

sed 's/id="org.gnome.Resources"/id="net.nokyan.Resources"/g' "${BUILD_DIR}/data/org.gnome.Resources.gschema.xml" > /usr/share/glib-2.0/schemas/net.nokyan.Resources.gschema.xml
chmod 644 /usr/share/glib-2.0/schemas/net.nokyan.Resources.gschema.xml

glib-compile-schemas /usr/share/glib-2.0/schemas/

echo "==> Installing desktop launchers..."
if [ -f "${BUILD_DIR}/data/org.gnome.Resources.desktop" ]; then
  cp -f "${BUILD_DIR}/data/org.gnome.Resources.desktop" /usr/share/applications/org.gnome.Resources.desktop
  cp -f "${BUILD_DIR}/data/org.gnome.Resources.desktop" /usr/share/applications/net.nokyan.Resources.desktop
  chmod 644 /usr/share/applications/org.gnome.Resources.desktop /usr/share/applications/net.nokyan.Resources.desktop
fi

echo "==> Installing icons..."
if [ -d "${SCRIPT_DIR}/data/icons" ]; then
  mkdir -p /usr/share/icons/hicolor/scalable/apps
  cp ${SCRIPT_DIR}/data/icons/*.svg /usr/share/icons/hicolor/scalable/apps/ 2>/dev/null || true
fi

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
