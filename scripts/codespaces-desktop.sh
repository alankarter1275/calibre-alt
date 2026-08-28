#!/usr/bin/env bash
set -euo pipefail

STATE_DIR="${HOME}/.cache/kalam-codespace"
LOG_DIR="${STATE_DIR}/logs"
ENV_FILE="${STATE_DIR}/env.sh"
DISPLAY_NUMBER="${DISPLAY_NUMBER:-1}"
DISPLAY_VALUE=":${DISPLAY_NUMBER}"
SCREEN_SIZE="${SCREEN_SIZE:-1600x900x24}"
VNC_PORT="${VNC_PORT:-5901}"
NOVNC_PORT="${NOVNC_PORT:-6080}"
DBUS_ADDRESS="unix:path=${STATE_DIR}/dbus-session"
NOVNC_SOURCE_DIR="/usr/share/novnc"
NOVNC_WEB_DIR="${STATE_DIR}/novnc-site"
RUNTIME_DIR="${STATE_DIR}/runtime"

mkdir -p "${LOG_DIR}" "${NOVNC_WEB_DIR}" "${RUNTIME_DIR}"
chmod 700 "${RUNTIME_DIR}"

export DISPLAY="${DISPLAY_VALUE}"
export DBUS_SESSION_BUS_ADDRESS="${DBUS_ADDRESS}"
export XDG_RUNTIME_DIR="${RUNTIME_DIR}"
export LIBGL_ALWAYS_SOFTWARE=1
export GSK_RENDERER=cairo
export WEBKIT_DISABLE_DMABUF_RENDERER=1
export GTK_A11Y=none
export NO_AT_BRIDGE=1

cat >"${ENV_FILE}" <<EOF
export DISPLAY=${DISPLAY_VALUE}
export DBUS_SESSION_BUS_ADDRESS=${DBUS_ADDRESS}
export XDG_RUNTIME_DIR=${RUNTIME_DIR}
export LIBGL_ALWAYS_SOFTWARE=1
export GSK_RENDERER=cairo
export WEBKIT_DISABLE_DMABUF_RENDERER=1
export GTK_A11Y=none
export NO_AT_BRIDGE=1
EOF

cat >"${NOVNC_WEB_DIR}/index.html" <<'EOF'
<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <meta http-equiv="refresh" content="0; url=./vnc.html?autoconnect=1&resize=scale">
    <title>Kalam Desktop</title>
    <script>
      window.location.replace('./vnc.html?autoconnect=1&resize=scale');
    </script>
  </head>
  <body>
    Opening Kalam Desktop…
  </body>
</html>
EOF

for name in app core include utils vendor vnc.html vnc_lite.html vnc_auto.html; do
  if [ -e "${NOVNC_SOURCE_DIR}/${name}" ] && [ ! -e "${NOVNC_WEB_DIR}/${name}" ]; then
    ln -s "${NOVNC_SOURCE_DIR}/${name}" "${NOVNC_WEB_DIR}/${name}"
  fi
done

if [ ! -S "${STATE_DIR}/dbus-session" ]; then
  dbus-daemon --session --address="${DBUS_ADDRESS}" --fork --nopidfile
fi

if ! pgrep -f "Xvfb ${DISPLAY_VALUE}" >/dev/null 2>&1; then
  Xvfb "${DISPLAY_VALUE}" -screen 0 "${SCREEN_SIZE}" -ac +render -noreset \
    >"${LOG_DIR}/xvfb.log" 2>&1 &
fi

sleep 1

if ! pgrep -u "${USER}" -f "openbox" >/dev/null 2>&1; then
  env DISPLAY="${DISPLAY_VALUE}" DBUS_SESSION_BUS_ADDRESS="${DBUS_ADDRESS}" \
    openbox >"${LOG_DIR}/openbox.log" 2>&1 &
fi

if ! pgrep -f "x11vnc .*${DISPLAY_VALUE}.*${VNC_PORT}" >/dev/null 2>&1; then
  x11vnc -display "${DISPLAY_VALUE}" -rfbport "${VNC_PORT}" -forever -shared -nopw -bg \
    -o "${LOG_DIR}/x11vnc.log" >/dev/null 2>&1
fi

existing_novnc_pids="$(pgrep -f "websockify .*${NOVNC_PORT}.*${VNC_PORT}" || true)"
if [ -n "${existing_novnc_pids}" ]; then
  kill ${existing_novnc_pids} >/dev/null 2>&1 || true
  sleep 1
fi

websockify --web="${NOVNC_WEB_DIR}" "${NOVNC_PORT}" "localhost:${VNC_PORT}" \
  >"${LOG_DIR}/novnc.log" 2>&1 &

echo
if [ -n "${CODESPACE_NAME:-}" ]; then
  echo "Desktop link: https://${CODESPACE_NAME}-${NOVNC_PORT}.app.github.dev/vnc.html?autoconnect=1&resize=scale"
fi
echo "Desktop is ready."
echo ""
echo "Next steps:"
echo "1. Open the Ports tab in Codespaces."
echo "2. Open port ${NOVNC_PORT} in your browser."
echo "3. In the terminal, run:"
echo "   source ${ENV_FILE}"
echo "   cargo run"
echo ""
echo "If you want one command, run:"
echo "   ./scripts/run-kalam-codespace.sh"
