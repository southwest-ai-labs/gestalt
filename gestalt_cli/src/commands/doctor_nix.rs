use std::env;
use std::path::Path;

pub struct NixCheckResult {
    pub is_nixos: bool,
    pub missing_vars: Vec<String>,
    pub dev_shell_active: bool,
}

pub fn check_nixos_environment() -> NixCheckResult {
    let is_nixos = is_nixos_system();
    check_environment(is_nixos)
}

fn is_nixos_system() -> bool {
    Path::new("/etc/NIXOS").exists() || Path::new("/run/current-system/sw/bin/nixos-version").exists()
}

pub fn check_environment(is_nixos: bool) -> NixCheckResult {
    let mut missing_vars = Vec::new();
    if is_nixos {
        for var in &["OPENSSL_DIR", "OPENSSL_LIB_DIR", "PKG_CONFIG_PATH"] {
            if env::var(var).is_err() {
                missing_vars.push(var.to_string());
            }
        }
    }
    
    let dev_shell_active = env::var("IN_NIX_SHELL").is_ok() || env::var("DIRENV_DIR").is_ok();
    
    NixCheckResult {
        is_nixos,
        missing_vars,
        dev_shell_active,
    }
}
