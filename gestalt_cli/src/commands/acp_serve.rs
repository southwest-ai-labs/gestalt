use tracing::{error, info};

/// Resolve the `gestalt_acp` binary path.
///
/// Resolves `gestalt_acp` relative to `current_exe().parent()` (accounting for
/// `.exe` on Windows), falling back to an explicitly configured `GESTALT_ACP_BIN`
/// environment variable only if it is an absolute path to an existing file.
pub fn resolve_acp_binary() -> anyhow::Result<std::path::PathBuf> {
    let current_exe = std::env::current_exe().ok();
    let env_bin = std::env::var_os("GESTALT_ACP_BIN").map(std::path::PathBuf::from);
    resolve_acp_binary_impl(current_exe.as_deref(), env_bin.as_deref())
}

/// Core binary resolution logic decoupled from system environment for testability.
pub fn resolve_acp_binary_impl(
    current_exe: Option<&std::path::Path>,
    env_bin: Option<&std::path::Path>,
) -> anyhow::Result<std::path::PathBuf> {
    let bin_name = if cfg!(windows) {
        "gestalt_acp.exe"
    } else {
        "gestalt_acp"
    };
    resolve_acp_binary_with_name(current_exe, env_bin, bin_name)
}

/// Helper function parameterized with target binary name to allow cross-platform testing.
pub fn resolve_acp_binary_with_name(
    current_exe: Option<&std::path::Path>,
    env_bin: Option<&std::path::Path>,
    bin_name: &str,
) -> anyhow::Result<std::path::PathBuf> {
    if let Some(exe) = current_exe {
        if let Some(parent) = exe.parent() {
            let candidate = parent.join(bin_name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }

    if let Some(env_path) = env_bin {
        if !env_path.is_absolute() {
            anyhow::bail!(
                "GESTALT_ACP_BIN must be an absolute path, got: {}",
                env_path.display()
            );
        }
        if !env_path.is_file() {
            anyhow::bail!(
                "GESTALT_ACP_BIN points to non-existent file: {}",
                env_path.display()
            );
        }
        return Ok(env_path.to_path_buf());
    }

    let candidate_desc = current_exe
        .and_then(|p| p.parent().map(|d| d.join(bin_name)))
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| bin_name.to_string());

    anyhow::bail!(
        "Failed to resolve {bin_name} binary: candidate '{candidate_desc}' does not exist and GESTALT_ACP_BIN is unset or invalid"
    )
}

/// Evaluate child exit status, logging exit details and returning Err on failure.
pub(crate) fn evaluate_exit_status(
    status_opt: Option<std::process::ExitStatus>,
    shutdown: &crate::shutdown::ShutdownCoordinator,
) -> anyhow::Result<()> {
    match status_opt {
        Some(status) => {
            if !status.success() {
                #[cfg(unix)]
                let exit_desc = match (
                    status.code(),
                    std::os::unix::process::ExitStatusExt::signal(&status),
                ) {
                    (Some(code), _) => format!("exit code {code}"),
                    (None, Some(sig)) => format!("signal {sig}"),
                    (None, None) => "unknown termination reason".to_string(),
                };
                #[cfg(not(unix))]
                let exit_desc = match status.code() {
                    Some(code) => format!("exit code {code}"),
                    None => "unknown termination reason".to_string(),
                };

                error!("ACP server exited with error ({exit_desc})");
                return Err(anyhow::anyhow!(
                    "ACP server exited with error ({exit_desc})"
                ));
            }
            Ok(())
        },
        None => {
            shutdown.record_cancelled(1);
            Ok(())
        },
    }
}

pub async fn serve(
    stdio: bool,
    port: Option<u16>,
    shutdown: &crate::shutdown::ShutdownCoordinator,
) -> anyhow::Result<()> {
    let acp_bin = resolve_acp_binary()?;
    serve_with_command(stdio, port, shutdown, acp_bin).await
}

pub(crate) async fn serve_with_command(
    stdio: bool,
    port: Option<u16>,
    shutdown: &crate::shutdown::ShutdownCoordinator,
    acp_bin: std::path::PathBuf,
) -> anyhow::Result<()> {
    let transport = if stdio { "stdio" } else { "http" };
    let port = port.unwrap_or(8082);

    info!("Starting ACP Server...");
    if !stdio {
        eprintln!("🚀 Starting Standalone ACP Server ({})", transport);
    }

    let mut cmd = tokio::process::Command::new(acp_bin);
    cmd.kill_on_drop(true);
    cmd.arg("--transport").arg(transport);
    if !stdio {
        cmd.arg("--port").arg(port.to_string());
    }

    let mut child = cmd.spawn()?;

    let wait_res =
        crate::wait_for_child_or_shutdown(&mut child, shutdown, "standalone ACP server").await?;
    evaluate_exit_status(wait_res, shutdown)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn test_resolve_acp_binary_sibling_exists() {
        let dir = tempdir().expect("create tempdir");
        let fake_exe = dir.path().join("gestalt");
        File::create(&fake_exe).expect("create fake exe");

        let fake_acp = dir.path().join("gestalt_acp");
        File::create(&fake_acp).expect("create fake acp");

        let resolved = resolve_acp_binary_with_name(Some(&fake_exe), None, "gestalt_acp")
            .expect("resolution should succeed");
        assert_eq!(resolved, fake_acp);
    }

    #[test]
    fn test_resolve_acp_binary_windows_extension() {
        let dir = tempdir().expect("create tempdir");
        let fake_exe = dir.path().join("gestalt.exe");
        File::create(&fake_exe).expect("create fake exe");

        let fake_acp_win = dir.path().join("gestalt_acp.exe");
        File::create(&fake_acp_win).expect("create fake acp win");

        let resolved = resolve_acp_binary_with_name(Some(&fake_exe), None, "gestalt_acp.exe")
            .expect("resolution should succeed with windows extension");
        assert_eq!(resolved, fake_acp_win);
    }

    #[test]
    fn test_resolve_acp_binary_fallback_env_valid() {
        let dir = tempdir().expect("create tempdir");
        let custom_bin = dir.path().join("custom_gestalt_acp");
        File::create(&custom_bin).expect("create custom acp");

        let empty_exe_dir = tempdir().expect("empty dir");
        let fake_exe = empty_exe_dir.path().join("gestalt");

        let resolved = resolve_acp_binary_impl(Some(&fake_exe), Some(&custom_bin))
            .expect("fallback env resolution should succeed");
        assert_eq!(resolved, custom_bin);
    }

    #[test]
    fn test_resolve_acp_binary_fallback_env_relative_rejected() {
        let relative_path = Path::new("relative/gestalt_acp");
        let err = resolve_acp_binary_impl(None, Some(relative_path))
            .expect_err("relative path should fail");
        assert!(err.to_string().contains("must be an absolute path"));
    }

    #[test]
    fn test_resolve_acp_binary_fallback_env_nonexistent_rejected() {
        let nonexistent = Path::new("/tmp/nonexistent_gestalt_acp_binary_test_path");
        let err = resolve_acp_binary_impl(None, Some(nonexistent))
            .expect_err("nonexistent file should fail");
        assert!(err.to_string().contains("points to non-existent file"));
    }

    #[test]
    fn test_resolve_acp_binary_none_available() {
        let empty_exe_dir = tempdir().expect("empty dir");
        let fake_exe = empty_exe_dir.path().join("gestalt");
        let err = resolve_acp_binary_impl(Some(&fake_exe), None)
            .expect_err("should fail when sibling missing and env unset");
        assert!(err.to_string().contains("Failed to resolve"));
    }

    #[test]
    fn test_evaluate_exit_status_none_cancelled() {
        let shutdown = crate::shutdown::ShutdownCoordinator::default();
        let res = evaluate_exit_status(None, &shutdown);
        assert!(res.is_ok());
        let report = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(shutdown.drain(std::time::Duration::from_millis(50)));
        assert_eq!(report.cancelled, 1);
    }

    #[tokio::test]
    async fn test_error_propagation_on_nonzero_child_exit() {
        let dir = tempdir().expect("tempdir");
        let script_path = dir.path().join("failing_acp.sh");

        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::PermissionsExt;

            let mut file = File::create(&script_path).expect("create script");
            writeln!(file, "#!/bin/sh\nexit 42").expect("write script");
            file.flush().expect("flush");

            let mut perms = std::fs::metadata(&script_path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script_path, perms).expect("set permissions");
        }

        #[cfg(windows)]
        {
            use std::io::Write;
            let mut file = File::create(&script_path).expect("create batch");
            writeln!(file, "@exit 42").expect("write batch");
            file.flush().expect("flush");
        }

        let shutdown = crate::shutdown::ShutdownCoordinator::default();
        let result = serve_with_command(false, Some(9099), &shutdown, script_path).await;
        assert!(
            result.is_err(),
            "expected serve_with_command to fail on non-zero exit code"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("exit code 42"),
            "expected error message to mention 'exit code 42', got: '{err_msg}'"
        );
    }
}
