#!/bin/sh
# Support `curl ... | sh` by re-executing under bash (this script uses bash features).
if [ -z "${BASH_VERSION:-}" ]; then
  if ! command -v bash >/dev/null 2>&1; then
    echo "bash is required to install Tiles" >&2
    exit 1
  fi
  # File invocation: re-exec the same path with bash.
  if [ -f "$0" ] && [ -r "$0" ]; then
    case "$0" in
      sh|-sh|*/sh) ;;
      *) exec bash "$0" "$@" ;;
    esac
  fi
  # Piped invocation (`curl | sh`): remaining stdin is the rest of this script.
  # Avoid `exec` + EXIT trap (that can delete the temp file before bash starts).
  _tiles_install_tmp="$(mktemp)"
  cat > "${_tiles_install_tmp}"
  bash "${_tiles_install_tmp}" "$@"
  _tiles_install_status=$?
  rm -f "${_tiles_install_tmp}"
  exit "${_tiles_install_status}"
fi

set -euo pipefail

REPO="tilesprivacy/tiles"

# the latest release unless --version picks one
VERSION=""
DEV="false"
NIGHTLY="false"
BACKEND=""
HEADLESS="false"
INSTALL_DIR_OVERRIDE=""
LIB_DIR_OVERRIDE=""

# how the log is drawn. plain lines unless detect_terminal_style finds a terminal
LIVE="false"
BOLD="" DIM="" YELLOW="" TRACK="" RESET=""
CHIP_READY="" CHIP_WARN="" CHIP_FAIL=""
MARK_FRAMES=(". " ".." " ." "  ")
BAR_FILL="########################"
BAR_TRACK="------------------------"

# the step whose line is open, and the file its output goes to
STEP_VERB=""
STEP_DETAIL=""
STEP_CLOCK=""
STEP_LOG=""
# the background job of the open step, while it runs
STEP_JOB=""
WARNINGS=0
# true from the header until the install first changes something
UNTOUCHED="false"

# colour and redrawing need a terminal at least as wide as the log
detect_terminal_style() {
  local cols locale="${LC_ALL:-${LC_CTYPE:-${LANG:-}}}"
  if [[ ! -t 1 || "${TERM:-dumb}" == "dumb" || -n "${NO_COLOR:-}" ]]; then
    return 0
  fi
  cols="$({ stty size < /dev/tty; } 2>/dev/null | awk '{ print $2 }' || true)"
  if [[ ! "${cols}" =~ ^[0-9]+$ ]]; then
    cols=80
  fi
  if [[ "${cols}" -lt 64 ]]; then
    return 0
  fi

  LIVE="true"
  BOLD=$'\033[1m'
  RESET=$'\033[0m'
  CHIP_WARN=$'\033[1;7m'
  if [[ "${TERM}" == *256color* || -n "${COLORTERM:-}" ]]; then
    DIM=$'\033[38;5;245m'
    # Tiles yellow sits on a dark cell, so a light terminal can read it
    YELLOW=$'\033[38;5;227;48;5;233m'
    TRACK=$'\033[38;5;238;48;5;233m'
    CHIP_READY=$'\033[1;38;5;233;48;5;227m'
    CHIP_FAIL=$'\033[1;38;5;233;48;5;203m'
  else
    DIM=$'\033[2m'
    YELLOW=$'\033[33m'
    TRACK=$'\033[2m'
    CHIP_READY=$'\033[1;30;43m'
    CHIP_FAIL=$'\033[1;30;41m'
  fi

  case "${locale}" in
    *UTF-8*|*utf-8*|*UTF8*|*utf8*) ;;
    *) return 0 ;;
  esac
  # the linux console font has no heavy line
  if [[ "${TERM}" == "linux" ]]; then
    return 0
  fi
  MARK_FRAMES=("▀ " " ▀" " ▄" "▄ ")
  BAR_FILL="━━━━━━━━━━━━━━━━━━━━━━━━"
  BAR_TRACK="${BAR_FILL}"
}

# the time since the install began, as MM:SS
print_install_clock() {
  printf '%02d:%02d' $((SECONDS / 60)) $((SECONDS % 60))
}

# a path with the home folder written as ~
print_short_path() {
  if [[ "$1" == "${HOME}"/* ]]; then
    printf '~%s' "${1#"${HOME}"}"
  else
    printf '%s' "$1"
  fi
}

# one log line: a clock, a verb, what it's about, and a status box at column 57
draw_log_line() {
  local clock="$1" verb="$2" detail="$3" box="$4"
  if [[ "${LIVE}" == "true" ]]; then
    printf '\r\033[K %s[%s]%s %s%-7s%s %s\033[57G%s' \
      "${DIM}" "${clock}" "${RESET}" "${BOLD}" "${verb}" "${RESET}" "${detail}" "${box}"
  else
    printf ' [%s] %-7s %-38s %s' "${clock}" "${verb}" "${detail}" "${box}"
  fi
}

# on a terminal a detail longer than its column would run under the status box
print_clipped_detail() {
  if [[ "${LIVE}" == "true" && ${#1} -gt 38 ]]; then
    printf '%s...' "${1:0:35}"
  else
    printf '%s' "$1"
  fi
}

# the open line with its mark, which moves on every second tick
draw_running_step() {
  local detail="$1" tick="$2"
  draw_log_line "${STEP_CLOCK}" "${STEP_VERB}" "${detail}" \
    "${DIM}[${RESET} ${YELLOW}${MARK_FRAMES[$(((tick / 2) % 4))]}${RESET} ${DIM}]${RESET}"
}

# opens a log line for the work that follows
begin_step() {
  STEP_VERB="$1"
  STEP_DETAIL="$(print_clipped_detail "$2")"
  STEP_CLOCK="$(print_install_clock)"
  if [[ "${LIVE}" == "true" ]]; then
    draw_running_step "${STEP_DETAIL}" 0
  fi
}

# closes the open line as ok, warn or fail. a second argument replaces its detail
end_step() {
  local box detail
  detail="$(print_clipped_detail "${2:-${STEP_DETAIL}}")"
  case "$1" in
    ok) box="${DIM}[${RESET} OK ${DIM}]${RESET}" ;;
    warn)
      box="${CHIP_WARN} WARN ${RESET}"
      WARNINGS=$((WARNINGS + 1))
      ;;
    *) box="${CHIP_FAIL} FAIL ${RESET}" ;;
  esac
  # without colour a chip is only a word, so it gets brackets
  if [[ "${LIVE}" != "true" && "$1" != "ok" ]]; then
    box="[${box// /}]"
  fi
  draw_log_line "${STEP_CLOCK}" "${STEP_VERB}" "${detail}" "${box}"
  printf '\n'
  STEP_VERB=""
}

# ctrl-c doesn't reach a background job, it would go on writing files
stop_step_job() {
  if [[ -n "${STEP_JOB}" ]]; then
    kill "${STEP_JOB}" 2>/dev/null || true
    wait "${STEP_JOB}" 2>/dev/null || true
    STEP_JOB=""
  fi
}

# a step still open when the script exits is the one that failed
close_open_step() {
  stop_step_job
  if [[ -n "${STEP_VERB}" ]]; then
    end_step fail
  fi
}
trap close_open_step EXIT

# a line under a step, in its detail column. a second argument is its colour
print_step_note() {
  printf '%17s%s%s%s\n' "" "${2:-}" "$1" "${RESET}"
}

# a line whose verb is a filled label, like READY
print_badge_line() {
  local chip="$1" word="$2" message="$3"
  printf ' %s[%s]%s %s %s %s%*s %s\n' "${DIM}" "$(print_install_clock)" "${RESET}" \
    "${chip}" "${word}" "${RESET}" $((5 - ${#word})) "" "${message}"
}

# a warning of its own, outside any step
print_warning() {
  print_badge_line "${CHIP_WARN}" WARN "$1"
  WARNINGS=$((WARNINGS + 1))
}

err() {
  if [[ -n "${STEP_VERB}" ]]; then
    end_step fail
  fi
  print_badge_line "${CHIP_FAIL}" FAIL "$*" >&2
  if [[ "${UNTOUCHED}" == "true" ]]; then
    print_step_note "nothing was changed" "${DIM}" >&2
  fi
  exit 1
}

print_step_detail() {
  printf '%s' "${STEP_DETAIL}"
}

# the download so far, as a bar and megabytes
print_download_progress() {
  local file="${TMPDIR}/tiles.tar.gz" got=0 total=0 filled
  if [[ -f "${file}" ]]; then
    got=$(($(wc -c < "${file}")))
  fi
  # redirects answer first, so the last length is the file's
  if [[ -f "${TMPDIR}/headers" ]]; then
    total="$(awk 'tolower($1) == "content-length:" { n = $2 } END { print n + 0 }' "${TMPDIR}/headers")"
  fi
  if [[ "${total}" -gt 0 ]]; then
    filled=$((got * 24 / total))
    if [[ "${filled}" -gt 24 ]]; then
      filled=24
    fi
    printf '%s%s%s%s%s %3d/%d MB' "${YELLOW}" "${BAR_FILL:0:filled}" "${TRACK}" "${BAR_TRACK:filled}" "${RESET}" \
      $((got / 1048576)) $((total / 1048576))
  else
    printf '%d MB' $((got / 1048576))
  fi
}

# the files the open step has unpacked so far, like 4,406
print_unpacked_count() {
  local files
  files=$(($(wc -l < "${STEP_LOG}")))
  if [[ "${files}" -ge 1000 ]]; then
    printf '%s  %d,%03d files' "${STEP_DETAIL}" $((files / 1000)) $((files % 1000))
  else
    printf '%s  %d files' "${STEP_DETAIL}" "${files}"
  fi
}

# runs a command in the background as the open step and redraws its line until it ends.
# the first argument is the function that prints the line's detail
run_step_job() {
  local print_detail="$1" pid tick=0
  shift
  "$@" > "${STEP_LOG}" 2>&1 &
  pid=$!
  STEP_JOB="${pid}"
  while kill -0 "${pid}" 2>/dev/null; do
    if [[ "${LIVE}" == "true" ]]; then
      draw_running_step "$("${print_detail}")" "${tick}"
    fi
    tick=$((tick + 1))
    sleep 0.1
  done
  STEP_JOB=""
  if wait "${pid}"; then
    return 0
  fi
  end_step fail
  tail -n 5 "${STEP_LOG}" >&2
  return 1
}

# what is being installed. the version shows once it's known
print_install_header() {
  local title="${BOLD}Tiles${RESET}"
  if [[ -n "${VERSION}" ]]; then
    title="${title}  ${VERSION}"
  fi
  printf ' %s  %sinstaller for %s-%s%s\n\n' "${title}" "${DIM}" "${ARCH}" "${OS}" "${RESET}"
  UNTOUCHED="true"
}

detect_terminal_style

usage() {
  echo "Usage: curl -LsSf https://www.tiles.run/install.sh | sh -s -- [OPTIONS]"
  echo "   or: scripts/install.sh [OPTIONS]"
  echo ""
  echo "  --dev                Install from a local dist/*.tar.gz instead of GitHub"
  echo "  --version VERSION    Install that release, like an rc (default: the latest release)"
  echo "  --nightly            Install the latest nightly GitHub release"
  echo "                       (e.g. tiles-v0.4.17-x86_64-linux.tar.gz)"
  echo "  --headless           Leave out the Tiles app and the web UI, for servers and Tilekit"
  echo "  --install-dir PATH   Override the binary installation directory"
  echo "  --lib-dir PATH       Override the runtime installation directory"
}

# github.com redirects /releases/latest to the newest release that isn't a pre-release
latest_version() {
  local location
  location="$(curl -fsSI "https://github.com/${REPO}/releases/latest" | tr -d '\r' | grep -i '^location:' || true)"
  [[ "${location}" == */releases/tag/* ]] || return 1
  echo "${location##*/}"
}

# Resolve the newest GitHub nightly release that has a tarball for this platform.
# The asset version may differ from the tag, e.g. tag 0.4.17-nightly.2 shipping
# a nightly.1 Linux tarball.
# Sets RELEASE_TAG, VERSION (for logs), and RELEASE_ASSET (exact filename).
resolve_nightly_version() {
  local api_url="https://api.github.com/repos/${REPO}/releases?per_page=30"
  local releases_json tag tags release_json asset
  local platform="${ARCH}-${OS}"

  releases_json="$(curl -fsSL "${api_url}")" || err "Failed to query GitHub releases for ${REPO}."

  tags="$(
    printf '%s' "${releases_json}" \
      | grep -oE '"tag_name"[[:space:]]*:[[:space:]]*"[^"]+"' \
      | sed -E 's/.*"([^"]+)".*/\1/' \
      | grep -i 'nightly' \
      || true
  )"

  # Walk nightly tags (API returns newest first) and inspect that release's assets.
  while IFS= read -r tag; do
    [ -z "${tag}" ] && continue
    release_json="$(
      curl -fsSL "https://api.github.com/repos/${REPO}/releases/tags/${tag}" 2>/dev/null || true
    )"
    [ -z "${release_json}" ] && continue

    asset="$(
      printf '%s' "${release_json}" \
        | grep -oE "\"name\"[[:space:]]*:[[:space:]]*\"tiles-v[^\"]*-${platform}\\.tar\\.gz\"" \
        | sed -E 's/.*"([^"]+)".*/\1/' \
        | head -n 1 \
        || true
    )"
    [ -z "${asset}" ] && continue

    RELEASE_TAG="${tag}"
    VERSION="${tag#v}"
    RELEASE_ASSET="${asset}"
    return 0
  done <<EOF
${tags}
EOF

  err "No nightly found for ${platform} (no matching tiles-v*-${platform}.tar.gz on GitHub)."
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dev|-dev)
      DEV="true"
      ;;
    --nightly|-nightly)
      NIGHTLY="true"
      ;;
    --version)
      [[ $# -ge 2 && -n "$2" ]] || err "--version requires a value."
      VERSION="${2#v}"
      [[ -n "${VERSION}" ]] || err "--version requires a value."
      shift
      ;;
    --headless|-headless)
      HEADLESS="true"
      ;;
    --backend)
      [[ $# -ge 2 ]] || err "--backend requires a value."
      BACKEND="$2"
      shift
      ;;
    --install-dir)
      [[ $# -ge 2 ]] || err "--install-dir requires a value."
      [[ -n "$2" ]] || err "--install-dir requires a non-empty value."
      INSTALL_DIR_OVERRIDE="$2"
      shift
      ;;
    --lib-dir)
      [[ $# -ge 2 ]] || err "--lib-dir requires a value."
      [[ -n "$2" ]] || err "--lib-dir requires a non-empty value."
      LIB_DIR_OVERRIDE="$2"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
  shift
done

if [[ "${DEV}" == "true" && "${NIGHTLY}" == "true" ]]; then
  err "--dev and --nightly cannot be used together."
fi
if [[ -n "${VERSION}" && ( "${DEV}" == "true" || "${NIGHTLY}" == "true" ) ]]; then
  err "--version cannot be used with --dev or --nightly."
fi

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

[[ "${OS}" == "linux" || "${OS}" == "darwin" ]] || err "Unsupported OS: ${OS}."

if [[ "${OS}" == "linux" && "$(id -u)" != "0" ]]; then
  INSTALL_DIR="${HOME}/.local/bin"
  LIB_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}/tiles"
else
  INSTALL_DIR="/usr/local/bin"
  LIB_DIR="/usr/local/share/tiles"
fi

[[ -n "${INSTALL_DIR_OVERRIDE}" ]] && INSTALL_DIR="${INSTALL_DIR_OVERRIDE}"
[[ -n "${LIB_DIR_OVERRIDE}" ]] && LIB_DIR="${LIB_DIR_OVERRIDE}"

SERVER_DIR="${LIB_DIR}/server"         # Python server folder
MODELFILE_DIR="${LIB_DIR}/modelfiles"  # Modelfile server folder
PI_DIR="${LIB_DIR}/pi"
UI_DIR="${LIB_DIR}/ui"                 # web UI the daemon serves on port 1730

# the Tiles app. on Linux its launcher entry and icon go where the desktop looks for them
MAC_APP="/Applications/Tiles.app"
DESKTOP_BIN="${LIB_DIR}/tiles-desktop"
if [[ "$(id -u)" == "0" ]]; then
  SHARE_DIR="/usr/local/share"
else
  SHARE_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}"
fi
DESKTOP_ENTRY="${SHARE_DIR}/applications/tiles.desktop"
DESKTOP_ICON="${SHARE_DIR}/icons/hicolor/256x256/apps/tiles.png"

# the account, chats, models and settings
DATA_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}/tiles"
CONFIG_DIR="${XDG_CONFIG_HOME:-${HOME}/.config}/tiles"

# the version of the Tiles this install replaces, empty when there is none
installed_version() {
  local bin="${INSTALL_DIR}/tiles"
  [[ -x "${bin}" ]] || return 0
  "${bin}" --version 2>/dev/null | awk 'NR == 1 { print $NF }' || true
}

# a data folder the user picked themselves, outside DATA_DIR
custom_data_dir() {
  local path
  path="$(awk -F'"' '
    /^\[/ { in_data = ($0 == "[data]") }
    in_data && /^path[[:space:]]*=/ { print $2; exit }
  ' "${CONFIG_DIR}/config.toml" 2>/dev/null || true)"
  if [[ -n "${path}" && "${path}" != "${DATA_DIR}" && "${path}" != "${DATA_DIR}"/* ]]; then
    echo "${path}"
  fi
}

# Tiles 0.5 doesn't work on top of a 0.4 install, so the old one is removed first
confirm_remove_old() {
  local new="${VERSION:-0.5}" reply dir custom
  # `curl | sh` leaves stdin on the pipe, so the answer comes from the terminal
  if ! { true < /dev/tty; } 2>/dev/null; then
    err "Tiles ${OLD_VERSION} is installed, and Tiles ${new} can't be installed over it. Run this installer in a terminal to remove the old install first."
  fi

  custom="$(custom_data_dir)"
  {
    printf '\n %s WARN %s Tiles %s is installed on this computer.\n\n' "${CHIP_WARN}" "${RESET}" "${OLD_VERSION}"
    echo "Tiles ${new} has breaking changes and doesn't work on top of a 0.4 install,"
    echo "so the old install has to be removed completely first."
    echo ""
    echo "That deletes everything Tiles has saved in these folders, including your"
    echo "account, chats, settings and downloaded models:"
    echo ""
    for dir in "${OLD_DIRS[@]}"; do
      echo "  ${dir}"
    done
    echo ""
    if [[ -n "${custom}" ]]; then
      echo "Your data folder at ${custom} is left as it is."
      echo ""
    fi
    echo "Back up anything you want to keep before you continue."
    echo ""
    printf "Remove Tiles %s and install %s? [Y/n] " "${OLD_VERSION}" "${new}"
  } > /dev/tty

  read -r reply < /dev/tty || reply="n"
  case "${reply}" in
    ""|y|Y|yes|Yes|YES) ;;
    *)
      echo "Nothing was changed. Tiles ${OLD_VERSION} is still installed." > /dev/tty
      exit 1
      ;;
  esac
}

remove_old_install() {
  local dir
  begin_step clean "Tiles ${OLD_VERSION}"
  rm -f "${INSTALL_DIR}/tiles" "${DESKTOP_ENTRY}" "${DESKTOP_ICON}"
  for dir in "${OLD_DIRS[@]}"; do
    # only a folder named tiles, whatever --lib-dir or the environment points at
    if [[ "${dir}" == */tiles ]]; then
      rm -rf "${dir}"
    fi
  done
  end_step ok
  for dir in "${OLD_DIRS[@]}"; do
    if [[ "${dir}" != */tiles ]]; then
      print_step_note "left ${dir} as it is"
    fi
  done
}

print_install_header

if [[ -n "${BACKEND}" ]]; then
  print_warning "--backend is no longer needed"
  print_step_note "Tiles asks to download the GPU engine the first time it runs a model" "${DIM}"
fi

if [[ "${DEV}" == "false" && "${NIGHTLY}" == "false" && -z "${VERSION}" ]]; then
  begin_step link "github.com/${REPO}"
  VERSION="$(latest_version)" || err "Couldn't find the latest Tiles release on GitHub."
  end_step ok "github.com/${REPO}  ${VERSION}"
fi

TMPDIR="$(mktemp -d)"
STEP_LOG="${TMPDIR}/step.log"
RELEASE_TAG="${VERSION}"
RELEASE_ASSET="tiles-v${VERSION}-${ARCH}-${OS}.tar.gz"

if [[ "${NIGHTLY}" == "true" ]]; then
  begin_step link "github.com/${REPO}"
  resolve_nightly_version
  end_step ok "github.com/${REPO}  ${VERSION}"
fi

OLD_VERSION="$(installed_version)"
OLD_DIRS=("${DATA_DIR}" "${CONFIG_DIR}")
[[ "${LIB_DIR}" != "${DATA_DIR}" ]] && OLD_DIRS+=("${LIB_DIR}")
REMOVE_OLD="false"
if [[ "${OLD_VERSION}" == 0.4.* && "${VERSION}" != 0.4.* ]]; then
  confirm_remove_old
  REMOVE_OLD="true"
fi

if [[ "${DEV}" == "false" ]]; then
  # Stable (default) and --nightly both download from GitHub.
  # Unflagged install gets the latest release, or the one --version picks.
  # Nightly uses RELEASE_ASSET from resolve_nightly_version (may differ from tag).
  TAR_URL="https://github.com/${REPO}/releases/download/${RELEASE_TAG}/${RELEASE_ASSET}"
  begin_step fetch "${RELEASE_ASSET}"
  run_step_job print_download_progress curl -fsSL -D "${TMPDIR}/headers" -o "${TMPDIR}/tiles.tar.gz" "${TAR_URL}" \
    || err "Couldn't download ${RELEASE_ASSET}."
  end_step ok "${RELEASE_ASSET} $(($(wc -c < "${TMPDIR}/tiles.tar.gz") / 1048576))M"
else
  if [[ -n "${TILES_DEV_TARBALL:-}" ]]; then
    LOCAL_TARBALL="${TILES_DEV_TARBALL}"
  else
    LOCAL_TARBALL=""
    shopt -s nullglob
    LOCAL_TARBALLS=(dist/tiles-v*-"${ARCH}"-"${OS}".tar.gz)
    shopt -u nullglob
    for candidate in "${LOCAL_TARBALLS[@]}"; do
      if [[ -z "${LOCAL_TARBALL}" || "${candidate}" -nt "${LOCAL_TARBALL}" ]]; then
        LOCAL_TARBALL="${candidate}"
      fi
    done
  fi

  if [[ -z "${LOCAL_TARBALL}" || ! -f "${LOCAL_TARBALL}" ]]; then
    err "No local Tiles tarball found. Run just bundle first, or set TILES_DEV_TARBALL=/path/to/tar.gz."
  fi

  begin_step copy "${LOCAL_TARBALL}"
  cp "${LOCAL_TARBALL}" "${TMPDIR}/tiles.tar.gz"
  end_step ok
fi

# a daemon left running would keep serving the old version, so stop it first
stop_running_daemon() {
  local url="http://127.0.0.1:1729" tick
  curl -fsS -m 2 -o /dev/null "${url}/" 2>/dev/null || return 0

  begin_step halt "daemon on 127.0.0.1:1729"
  curl -fsS -m 5 -o /dev/null -X POST -H "Content-Type: application/json" "${url}/shutdown" 2>/dev/null \
    || curl -fsS -m 5 -o /dev/null "${url}/shutdown" 2>/dev/null \
    || true

  for tick in $(seq 40); do
    if ! curl -fsS -m 1 -o /dev/null "${url}/" 2>/dev/null; then
      end_step ok
      return 0
    fi
    if [[ "${LIVE}" == "true" ]]; then
      draw_running_step "${STEP_DETAIL}" $((tick * 2))
    fi
    sleep 0.25
  done
  end_step warn
  print_step_note "it is still running. run tiles daemon stop after the update"
}

UNTOUCHED="false"
stop_running_daemon

[[ "${REMOVE_OLD}" == "true" ]] && remove_old_install

begin_step unpack "release archive"
run_step_job print_step_detail tar -xzf "${TMPDIR}/tiles.tar.gz" -C "${TMPDIR}" \
  || err "Couldn't unpack the release archive."
end_step ok

begin_step write "$(print_short_path "${INSTALL_DIR}/tiles")"
mkdir -p "${INSTALL_DIR}"
install -m 755 "${TMPDIR}/tiles" "${INSTALL_DIR}/tiles"
end_step ok

FILES_DETAIL="$(($(ls "${TMPDIR}/modelfiles" | wc -l))) modelfiles, pi"
if [[ "${HEADLESS}" != "true" && -d "${TMPDIR}/ui" ]]; then
  FILES_DETAIL="${FILES_DETAIL}, web ui"
fi
begin_step write "${FILES_DETAIL}"
rm -rf "${MODELFILE_DIR}"

mkdir -p "${MODELFILE_DIR}"

cp -r "${TMPDIR}/modelfiles"/* "${MODELFILE_DIR}/"

rm -rf "${PI_DIR}"

mkdir -p "${PI_DIR}"

cp -r "${TMPDIR}/pi"/* "${PI_DIR}/"

# releases from before the web UI have no ui folder
if [[ "${HEADLESS}" == "true" ]]; then
  rm -rf "${UI_DIR}"
elif [[ -d "${TMPDIR}/ui" ]]; then
  rm -rf "${UI_DIR}"
  mkdir -p "${UI_DIR}"
  cp -r "${TMPDIR}/ui"/. "${UI_DIR}/"
fi

# remembered for `tiles update`, which runs this script again without the flag
if [[ "${HEADLESS}" == "true" ]]; then
  touch "${LIB_DIR}/.headless"
else
  rm -f "${LIB_DIR}/.headless"
fi
end_step ok

APP_INSTALLED="false"
WEBKIT_MISSING="false"
# a headless install also takes away an app an earlier install left
if [[ "${HEADLESS}" == "true" ]]; then
  begin_step skip "Tiles app and web ui, headless"
  [[ "${OS}" == "darwin" ]] && rm -rf "${MAC_APP}"
  rm -f "${DESKTOP_BIN}" "${DESKTOP_ENTRY}" "${DESKTOP_ICON}"
  end_step ok
elif [[ -d "${TMPDIR}/desktop" ]]; then
  if [[ "${OS}" == "darwin" ]]; then
    begin_step write "Tiles.app in Applications"
    rm -rf "${MAC_APP}"
    cp -R "${TMPDIR}/desktop/Tiles.app" "${MAC_APP}"
  else
    begin_step write "Tiles app in your launcher"
    install -m 755 "${TMPDIR}/desktop/tiles-desktop" "${DESKTOP_BIN}"
    mkdir -p "$(dirname "${DESKTOP_ENTRY}")" "$(dirname "${DESKTOP_ICON}")"
    install -m 644 "${TMPDIR}/desktop/tiles.png" "${DESKTOP_ICON}"
    # an icon cache another app left would hide the new icon
    HICOLOR_DIR="${SHARE_DIR}/icons/hicolor"
    touch "${HICOLOR_DIR}"
    gtk-update-icon-cache -q -f -t "${HICOLOR_DIR}" >/dev/null 2>&1 || true
    sed "s|^Exec=.*|Exec=\"${DESKTOP_BIN}\"|" "${TMPDIR}/desktop/tiles.desktop" > "${DESKTOP_ENTRY}"
    update-desktop-database "$(dirname "${DESKTOP_ENTRY}")" >/dev/null 2>&1 || true
    if command -v ldconfig >/dev/null 2>&1 \
      && ! ldconfig -p 2>/dev/null | grep 'libwebkit2gtk-4\.1\.so\.0' >/dev/null; then
      WEBKIT_MISSING="true"
    fi
  fi
  if [[ "${WEBKIT_MISSING}" == "true" ]]; then
    end_step warn
    print_step_note "WebKitGTK 4.1 is missing, the app needs it"
    print_step_note "apt libwebkit2gtk-4.1-0  dnf webkit2gtk4.1" "${DIM}"
    print_step_note "pacman webkit2gtk-4.1" "${DIM}"
    print_step_note "until then, tiles ui opens Tiles in your browser" "${DIM}"
  else
    end_step ok
  fi
  APP_INSTALLED="true"
fi

begin_step write "$(print_short_path "${SERVER_DIR}")"
rm -rf "${SERVER_DIR}"

mkdir -p "${SERVER_DIR}"

cp -r "${TMPDIR}/server"/* "${SERVER_DIR}/"

chmod +x "${SERVER_DIR}/bin/llama-server" 2>/dev/null || true
end_step ok

cd "${SERVER_DIR}/stack_export_prod"

for f in *.tar.xz; do
  # cpython3.13-linux_x86_64-1.tar.xz is shown as cpython3.13
  layer="${f%.tar.xz}"
  begin_step unpack "${layer%-*-*}"
  run_step_job print_unpacked_count tar -xvf "$f" || err "Couldn't unpack ${f}."
  end_step ok "$(print_unpacked_count)"
done

rm -rf *.tar.xz

# one job per command, a job has to be the process that gets stopped
begin_step link "python environment"
for layer in cpython3.13 app-server; do
  run_step_job print_step_detail "${layer}/bin/python" "${layer}/postinstall.py" \
    || err "Couldn't set up the Python environment."
done
end_step ok

rm -rf "${TMPDIR}"

case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    print_warning "$(print_short_path "${INSTALL_DIR}") is not in your PATH"
    print_step_note "add this line to ~/.bashrc or ~/.zshrc, then restart your terminal" "${DIM}"
    print_step_note "export PATH=$INSTALL_DIR:\$PATH"
    ;;
esac

READY_LINE="Tiles${VERSION:+ ${VERSION}} installed"
if [[ "${WARNINGS}" -eq 1 ]]; then
  READY_LINE="${READY_LINE}, with 1 warning"
elif [[ "${WARNINGS}" -gt 1 ]]; then
  READY_LINE="${READY_LINE}, with ${WARNINGS} warnings"
fi
print_badge_line "${CHIP_READY}" READY "${READY_LINE}"

if [[ "${APP_INSTALLED}" == "true" ]]; then
  printf '\n%s > %s Open Tiles from your apps, or run %stiles ui%s\n' "${YELLOW}" "${RESET}" "${BOLD}" "${RESET}"
else
  printf '\n%s > %s Start Tiles by running %stiles%s\n' "${YELLOW}" "${RESET}" "${BOLD}" "${RESET}"
fi
