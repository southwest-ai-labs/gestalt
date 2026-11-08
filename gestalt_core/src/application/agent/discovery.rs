use std::env;
use std::path::PathBuf;
use std::process::Command;

use crate::domain::capabilities::ProviderCapabilities;

#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredAgent {
    pub name: String,
    pub version: String,
    pub binary_path: PathBuf,
    pub capabilities: ProviderCapabilities,
}

pub fn discover_installed_clis() -> Vec<DiscoveredAgent> {
    let mut discovered = Vec::new();
    let known_clis = ["claude", "opencode", "agy", "hermes", "wrangler"];

    if let Some(paths) = env::var_os("PATH") {
        for dir in env::split_paths(&paths) {
            for cli in known_clis.iter() {
                let exe_path = dir.join(cli);
                if exe_path.is_file() && is_executable(&exe_path) {
                    // Avoid duplicates if a CLI is found multiple times in PATH
                    if discovered.iter().any(|d: &DiscoveredAgent| &d.name == cli) {
                        continue;
                    }

                    let version = match Command::new(&exe_path).arg("--version").output() {
                        Ok(output) if output.status.success() => {
                            String::from_utf8_lossy(&output.stdout).trim().to_string()
                        }
                        _ => "unknown".to_string(),
                    };

                    let capabilities = match *cli {
                        "claude" => ProviderCapabilities::claude(),
                        "opencode" => ProviderCapabilities::opencode(),
                        "agy" => ProviderCapabilities::agy(),
                        "wrangler" => ProviderCapabilities::wrangler(),
                        _ => ProviderCapabilities::default(),
                    };

                    discovered.push(DiscoveredAgent {
                        name: cli.to_string(),
                        version,
                        binary_path: exe_path,
                        capabilities,
                    });
                }
            }
        }
    }

    discovered
}

#[cfg(unix)]
fn is_executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = std::fs::metadata(path) {
        metadata.permissions().mode() & 0o111 != 0
    } else {
        false
    }
}

#[cfg(not(unix))]
fn is_executable(_path: &std::path::Path) -> bool {
    true
}
