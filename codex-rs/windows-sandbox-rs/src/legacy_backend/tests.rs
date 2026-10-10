#[path = "fixture.rs"]
mod fixture;

use crate::unified_exec::WindowsSandboxSessionRequest;
use crate::unified_exec::spawn_windows_sandbox_session_for_level;
use crate::unified_exec::spawn_windows_sandbox_session_legacy;
use codex_protocol::config_types::WindowsSandboxLevel;
use codex_protocol::models::PermissionProfile;
use codex_utils_absolute_path::AbsolutePathBuf;
use fixture::Fixture;

#[test]
fn capture_rejects_legacy_execution_before_deletion() {
    let fixture = Fixture::new();
    let cancellation = None;
    fixture.assert_rejected(crate::run_windows_sandbox_capture(
        &PermissionProfile::workspace_write(),
        &fixture.roots(),
        &fixture.home,
        fixture.command(),
        &fixture.workspace,
        fixture.environment(),
        Some(5_000),
        cancellation,
    ));
}

#[tokio::test]
async fn direct_sessions_reject_legacy_execution_with_or_without_a_tty() {
    for tty in [false, true] {
        let fixture = Fixture::new();
        let stdin_open = true;
        fixture.assert_rejected(
            spawn_windows_sandbox_session_legacy(
                &PermissionProfile::workspace_write(),
                &fixture.roots(),
                &fixture.home,
                fixture.command(),
                &fixture.workspace,
                fixture.environment(),
                Some(5_000),
                &[],
                &[],
                tty,
                stdin_open,
            )
            .await,
        );
    }
}

#[tokio::test]
async fn configured_sessions_reject_legacy_execution_for_both_permission_modes() {
    for profile in [
        PermissionProfile::workspace_write(),
        PermissionProfile::read_only(),
    ] {
        let fixture = Fixture::new();
        fixture.assert_rejected(
            spawn_windows_sandbox_session_for_level(WindowsSandboxSessionRequest {
                permission_profile: &profile,
                workspace_roots: &fixture.roots(),
                codex_home: &fixture.home,
                command: fixture.command(),
                cwd: &fixture.workspace,
                env_map: fixture.environment(),
                windows_sandbox_level: WindowsSandboxLevel::RestrictedToken,
                proxy_enforced: false,
                network_proxy_restricting_sid: None,
                proxy_settings_mode: crate::WindowsSandboxProxySettingsMode::Preserve,
                timeout_ms: Some(5_000),
                read_roots_override: None,
                read_roots_include_platform_defaults: false,
                write_roots_override: None,
                deny_read_paths_override: &[],
                deny_write_paths_override: &[],
                tty: false,
                stdin_open: false,
            })
            .await,
        );
    }
}

#[test]
fn wrapper_rejects_legacy_execution_before_preparing_a_desktop() {
    let fixture = Fixture::new();
    let mut environment = fixture.environment();
    let proxy_enforced = false;
    let network_proxy_restricting_sid = None;
    let read_roots_override = None;
    let read_roots_include_platform_defaults = false;
    let write_roots_override = None;
    fixture.assert_rejected(
        crate::create_windows_sandbox_command_args_for_permission_profile(
            fixture.command(),
            &AbsolutePathBuf::from_absolute_path(&fixture.workspace).unwrap(),
            &fixture.roots(),
            &mut environment,
            &PermissionProfile::workspace_write(),
            WindowsSandboxLevel::RestrictedToken,
            proxy_enforced,
            network_proxy_restricting_sid,
            crate::WindowsSandboxProxySettingsMode::Preserve,
            read_roots_override,
            read_roots_include_platform_defaults,
            write_roots_override,
            &[],
            &[],
            &fixture.home,
        ),
    );
}

#[test]
fn preflight_rejects_legacy_setup_before_changing_permissions() {
    let fixture = Fixture::new();
    fixture.assert_rejected(crate::run_windows_sandbox_legacy_preflight(
        &PermissionProfile::workspace_write(),
        &fixture.roots(),
        &fixture.home,
        &fixture.workspace,
        &fixture.environment(),
    ));
}
