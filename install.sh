#!/usr/bin/env bash
set -e

if [ "$EUID" -ne 0 ]; then
  echo "Please run as root using: sudo ./install.sh"
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="${SCRIPT_DIR}/build"
SUDO_USER_HOME=$(eval echo "~${SUDO_USER}")

echo "==> Stopping any running npu-data-exporter or resources processes..."
systemctl stop npu-data-exporter.service 2>/dev/null || true
pkill -9 npu-data-exporter 2>/dev/null || true
pkill -9 resources 2>/dev/null || true
pkill -9 resources-processes 2>/dev/null || true
sleep 1

echo "==> Installing updated resources binary..."
cp -f "${BUILD_DIR}/src/release/resources" /usr/bin/resources
chmod 755 /usr/bin/resources

echo "==> Installing companion helper binaries (resources-processes, adjust, kill)..."
mkdir -p /usr/libexec/resources
for bin in resources-processes resources-adjust resources-kill; do
  if [ -f "${BUILD_DIR}/src/release/${bin}" ]; then
    cp -f "${BUILD_DIR}/src/release/${bin}" "/usr/libexec/${bin}"
    cp -f "${BUILD_DIR}/src/release/${bin}" "/usr/libexec/resources/${bin}"
    chmod 755 "/usr/libexec/${bin}" "/usr/libexec/resources/${bin}"
  fi
done

echo "==> Installing npu-data-exporter binary..."
rm -f /usr/local/bin/npu-data-exporter
cp -f "${BUILD_DIR}/src/release/npu-data-exporter" /usr/local/bin/npu-data-exporter
chmod 755 /usr/local/bin/npu-data-exporter

echo "==> Installing compiled GResource..."
mkdir -p /usr/share/resources
cp -f "${BUILD_DIR}/data/resources/resources.gresource" /usr/share/resources/resources.gresource
chmod 644 /usr/share/resources/resources.gresource

echo "==> Installing compiled GSchemas to system and user locations..."
cp -f "${BUILD_DIR}/data/org.gnome.Resources.gschema.xml" /usr/share/glib-2.0/schemas/org.gnome.Resources.gschema.xml
chmod 644 /usr/share/glib-2.0/schemas/org.gnome.Resources.gschema.xml

sed 's/id="org.gnome.Resources"/id="net.nokyan.Resources"/g' "${BUILD_DIR}/data/org.gnome.Resources.gschema.xml" > /usr/share/glib-2.0/schemas/net.nokyan.Resources.gschema.xml
chmod 644 /usr/share/glib-2.0/schemas/net.nokyan.Resources.gschema.xml

glib-compile-schemas /usr/share/glib-2.0/schemas/

# If user has a local schema override dir, update it as well to prevent stale schema crashes
if [ -d "${SUDO_USER_HOME}/.local/share/glib-2.0/schemas" ]; then
  cp -f "${BUILD_DIR}/data/org.gnome.Resources.gschema.xml" "${SUDO_USER_HOME}/.local/share/glib-2.0/schemas/org.gnome.Resources.gschema.xml"
  chown "${SUDO_USER}:${SUDO_USER}" "${SUDO_USER_HOME}/.local/share/glib-2.0/schemas/org.gnome.Resources.gschema.xml" 2>/dev/null || true
  su - "${SUDO_USER}" -c "glib-compile-schemas '${SUDO_USER_HOME}/.local/share/glib-2.0/schemas'" 2>/dev/null || true
fi

echo "==> Installing desktop launchers with original theme icon (net.nokyan.Resources)..."
if [ -f "${BUILD_DIR}/data/org.gnome.Resources.desktop" ]; then
  # Preserve original Yaru icon name net.nokyan.Resources for system icon theme
  sed 's/Icon=org.gnome.Resources/Icon=net.nokyan.Resources/g' "${BUILD_DIR}/data/org.gnome.Resources.desktop" > /usr/share/applications/org.gnome.Resources.desktop
  cp -f /usr/share/applications/org.gnome.Resources.desktop /usr/share/applications/net.nokyan.Resources.desktop
  chmod 644 /usr/share/applications/org.gnome.Resources.desktop /usr/share/applications/net.nokyan.Resources.desktop
fi

echo "==> Installing icons..."
if [ -d "${SCRIPT_DIR}/data/icons" ]; then
  mkdir -p /usr/share/icons/hicolor/scalable/apps
  mkdir -p /usr/share/icons/hicolor/symbolic/apps
  cp ${SCRIPT_DIR}/data/icons/*-symbolic.svg /usr/share/icons/hicolor/symbolic/apps/ 2>/dev/null || true
  cp ${SCRIPT_DIR}/data/icons/*.svg /usr/share/icons/hicolor/scalable/apps/ 2>/dev/null || true
fi

echo "==> Installing UI translations (gettext locales)..."
if [ -d "${BUILD_DIR}/po" ]; then
  find "${BUILD_DIR}/po" -name "*.mo" | while read -r mo_file; do
    lang=$(echo "$mo_file" | sed -E 's|.*/po/([^/]+)/LC_MESSAGES/.*|\1|')
    if [ -n "$lang" ]; then
      mkdir -p "/usr/share/locale/${lang}/LC_MESSAGES"
      cp -f "$mo_file" "/usr/share/locale/${lang}/LC_MESSAGES/resources.mo"
      chmod 644 "/usr/share/locale/${lang}/LC_MESSAGES/resources.mo"
    fi
  done
fi

echo "==> Installing Polkit action policy..."
if [ -f "${BUILD_DIR}/data/org.gnome.Resources.policy" ]; then
  mkdir -p /usr/share/polkit-1/actions
  # Ensure the exec path points to /usr/libexec/resources/resources-kill
  sed 's|/tmp/resources-build/libexec/resources/resources-kill|/usr/libexec/resources/resources-kill|g' \
    "${BUILD_DIR}/data/org.gnome.Resources.policy" > /usr/share/polkit-1/actions/org.gnome.Resources.policy
  chmod 644 /usr/share/polkit-1/actions/org.gnome.Resources.policy

  # Provide legacy action ID net.nokyan.Resources.policy compatibility
  sed 's|id="org.gnome.Resources.kill"|id="net.nokyan.Resources.kill"|g' \
    /usr/share/polkit-1/actions/org.gnome.Resources.policy > /usr/share/polkit-1/actions/net.nokyan.Resources.policy
  chmod 644 /usr/share/polkit-1/actions/net.nokyan.Resources.policy
fi

echo "==> Installing npu-data-exporter systemd service..."
if command -v systemctl >/dev/null 2>&1 && [ -d /run/systemd/system ]; then
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
  systemctl enable --now npu-data-exporter.service 2>/dev/null || true
else
  echo "Notice: systemd is not active or available; skipping systemd service activation."
fi

echo "==> Installation complete! GNOME Resources and npu-data-exporter service are now active."
