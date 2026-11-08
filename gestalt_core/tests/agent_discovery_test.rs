use std::env;
use std::fs;
use tempfile::TempDir;

use gestalt_core::application::agent::discover_installed_clis;

#[test]
#[cfg(unix)]
fn test_discover_installed_clis() {
    use std::os::unix::fs::PermissionsExt;
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let clis = ["claude", "opencode", "agy", "hermes", "wrangler"];

    for cli in clis {
        let file_path = temp_path.join(cli);
        let script_content = format!("#!/bin/sh\necho \"mock-{}-1.0\"\n", cli);
        fs::write(&file_path, script_content).unwrap();

        // Make executable
        let mut perms = fs::metadata(&file_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&file_path, perms).unwrap();
    }

    // Save old path
    let old_path = env::var_os("PATH");
    env::set_var("PATH", temp_path);

    let discovered = discover_installed_clis();

    // Restore old path
    if let Some(p) = old_path {
        env::set_var("PATH", p);
    } else {
        env::remove_var("PATH");
    }

    assert_eq!(discovered.len(), 5);

    let names: Vec<String> = discovered.iter().map(|d| d.name.clone()).collect();
    assert!(names.contains(&"claude".to_string()));
    assert!(names.contains(&"opencode".to_string()));
    assert!(names.contains(&"agy".to_string()));
    assert!(names.contains(&"hermes".to_string()));
    assert!(names.contains(&"wrangler".to_string()));

    for agent in discovered {
        assert_eq!(agent.version, format!("mock-{}-1.0", agent.name));
        assert_eq!(agent.binary_path.parent().unwrap(), temp_path);
        
        // Basic check for capabilities based on name
        if agent.name == "claude" {
            assert!(agent.capabilities.vision);
        } else if agent.name == "wrangler" {
            assert!(!agent.capabilities.vision);
        }
    }
}
