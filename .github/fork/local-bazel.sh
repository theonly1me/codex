if [[ -z "${BUILDBUDDY_API_KEY:-}" ]]; then
  post_config_bazel_args+=(--disk_cache=)
  if [[ "${RUNNER_ENVIRONMENT:-}" == "github-hosted" ]]; then
    post_config_bazel_args+=(
      --@rules_rust//rust/settings:extra_rustc_flag=-Cdebuginfo=0
      --@rules_rust//rust/settings:extra_exec_rustc_flag=-Cdebuginfo=0
    )
    echo "Keyless hosted builds disable the disk cache and Rust debug information."
    if [[ "${RUNNER_OS:-}" != "Windows" && "${bazel_args[0]}" == "test" ]]; then
      post_config_bazel_args+=(--local_test_jobs=2 --test_env=RUST_TEST_THREADS=1)
      echo "Hosted local tests use two Bazel test jobs and one Rust test thread."
    fi
  fi
  if [[ "${RUNNER_OS:-}" == "Windows" ]]; then
    windows_test_config=ci-windows
    if [[ $windows_cross_compile -eq 1 ]]; then
      windows_test_config=ci-windows-cross
    fi
    while IFS= read -r argument; do
      post_config_bazel_args+=("$argument")
      printf 'Keyless Windows tests use the upstream setting: %s\n' "$argument"
    done < <(awk -v config="common:$windows_test_config" \
      '$1 == config && $2 ~ /^--test_env=/ { print $2 }' \
      "$(dirname "${BASH_SOURCE[0]}")/../../.bazelrc")
  fi
fi
