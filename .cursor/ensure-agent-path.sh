#!/usr/bin/env bash
# Rend uvx / scw / github-mcp-server visibles sans coller de PATH.
# Sourcé par install.sh et start.sh (idempotent).
export PATH="${HOME}/.local/bin:${HOME}/bin:/usr/local/bin:${PATH}"

_scola_link_bin() {
  local src="$1"
  local dest="$2"
  if [[ ! -x "$src" ]]; then
    return 0
  fi
  if [[ -x "$dest" ]]; then
    return 0
  fi
  if ln -sf "$src" "$dest" 2>/dev/null; then
    return 0
  fi
  if command -v sudo >/dev/null 2>&1; then
    sudo ln -sf "$src" "$dest" 2>/dev/null || true
  fi
}

_scola_link_bin "${HOME}/.local/bin/uvx" /usr/local/bin/uvx
_scola_link_bin "${HOME}/.local/bin/uv" /usr/local/bin/uv
_scola_link_bin "${HOME}/bin/scw" /usr/local/bin/scw
_scola_link_bin "${HOME}/.local/bin/scw" /usr/local/bin/scw
_scola_link_bin "${HOME}/.local/bin/github-mcp-server" /usr/local/bin/github-mcp-server
_scola_link_bin "${HOME}/bin/github-mcp-server" /usr/local/bin/github-mcp-server
