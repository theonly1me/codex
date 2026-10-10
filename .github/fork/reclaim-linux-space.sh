#!/usr/bin/env bash
set -euo pipefail

if [[ "${RUNNER_ENVIRONMENT:-}" != "github-hosted" || "${RUNNER_OS:-}" != "Linux" ]]; then
  echo "Disk reclamation requires a GitHub-hosted Linux runner." >&2
  exit 1
fi

for directory in /usr/local/lib/android /usr/share/dotnet /opt/ghc /usr/local/.ghcup; do
  if [[ -d "$directory" ]]; then
    sudo rm -rf -- "$directory"
  fi
done

df -h "${GITHUB_WORKSPACE:?}"
