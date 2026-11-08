#[path = "../src/commands/doctor_nix.rs"]
#[allow(dead_code)]
mod doctor_nix;

use doctor_nix::check_environment;
use std::env;

#[test]
fn test_nix_environment_checks() {
    // Save original env vars
    let orig_openssl_dir = env::var("OPENSSL_DIR").ok();
    let orig_openssl_lib_dir = env::var("OPENSSL_LIB_DIR").ok();
    let orig_pkg_config_path = env::var("PKG_CONFIG_PATH").ok();
    let orig_in_nix_shell = env::var("IN_NIX_SHELL").ok();
    let orig_direnv_dir = env::var("DIRENV_DIR").ok();
    
    // Test missing variables case on simulated NixOS
    env::remove_var("OPENSSL_DIR");
    env::remove_var("OPENSSL_LIB_DIR");
    env::remove_var("PKG_CONFIG_PATH");
    env::remove_var("IN_NIX_SHELL");
    env::remove_var("DIRENV_DIR");
    
    let result = check_environment(true);
    assert!(result.is_nixos);
    assert_eq!(result.missing_vars.len(), 3);
    assert!(!result.dev_shell_active);
    
    // Test healthy variables case on simulated NixOS
    env::set_var("OPENSSL_DIR", "/some/path");
    env::set_var("OPENSSL_LIB_DIR", "/some/lib/path");
    env::set_var("PKG_CONFIG_PATH", "/some/pkg/path");
    env::set_var("IN_NIX_SHELL", "impure");
    
    let result = check_environment(true);
    assert!(result.is_nixos);
    assert!(result.missing_vars.is_empty());
    assert!(result.dev_shell_active);
    
    // Test non-NixOS ignores missing vars
    env::remove_var("OPENSSL_DIR");
    env::remove_var("OPENSSL_LIB_DIR");
    env::remove_var("PKG_CONFIG_PATH");
    let result = check_environment(false);
    assert!(!result.is_nixos);
    assert!(result.missing_vars.is_empty());
    
    // Restore original env vars
    for (key, val) in [
        ("OPENSSL_DIR", orig_openssl_dir),
        ("OPENSSL_LIB_DIR", orig_openssl_lib_dir),
        ("PKG_CONFIG_PATH", orig_pkg_config_path),
        ("IN_NIX_SHELL", orig_in_nix_shell),
        ("DIRENV_DIR", orig_direnv_dir),
    ] {
        if let Some(v) = val {
            env::set_var(key, v);
        } else {
            env::remove_var(key);
        }
    }
}
