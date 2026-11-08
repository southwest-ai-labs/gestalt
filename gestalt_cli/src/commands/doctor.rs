use crate::commands::doctor_nix::check_nixos_environment;
use gestalt_core::application::agent::registry::AgentRegistry;
use gestalt_core::application::agent::xavier::XavierClient;
use gestalt_router::run_state::StateDb;
use std::time::Duration;

pub async fn run_doctor() {
    println!("🔍 Running Gestalt Doctor Environment Check...");
    println!("=============================================");
    let mut all_healthy = true;

    // 1. Xavier Reachability Check
    let xavier_client = XavierClient::from_env();
    match xavier_client.health().await {
        Ok(_) => {
            println!(
                "✅ Xavier reachability: Healthy (endpoint: {})",
                xavier_client.endpoint
            );
        },
        Err(e) => {
            println!(
                "❌ Xavier reachability: Unreachable (endpoint: {}): {}",
                xavier_client.endpoint, e
            );
            all_healthy = false;
        },
    }

    // 2. StateDb Open Check
    let db_path = home::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".gestalt")
        .join("state.db");
    match StateDb::open(&db_path) {
        Ok(_) => {
            println!("✅ StateDb open: Success (path: {})", db_path.display());
        },
        Err(e) => {
            println!(
                "❌ StateDb open: Failed (path: {}): {}",
                db_path.display(),
                e
            );
            all_healthy = false;
        },
    }

    // 3. Agent Registry Check
    let registry_path = AgentRegistry::resolve_path("agent-registry.toml");
    match AgentRegistry::load(&registry_path) {
        Ok(reg) => {
            println!(
                "✅ Agent registry parse: Success ({} agents loaded from {})",
                reg.agents.len(),
                registry_path.display()
            );
        },
        Err(e) => {
            println!(
                "❌ Agent registry parse: Failed to load (path: {}): {}",
                registry_path.display(),
                e
            );
            all_healthy = false;
        },
    }

    // 4. Bus Serve Reachability Check
    use std::net::TcpStream;
    match TcpStream::connect_timeout(
        &"127.0.0.1:8081".parse().unwrap(),
        Duration::from_secs(2),
    ) {
        Ok(_) => {
            println!("✅ Bus serve reachability: Reachable (port 8081)");
        },
        Err(e) => {
            println!("❌ Bus serve reachability: Unreachable (port 8081): {}", e);
            all_healthy = false;
        },
    }

    // 5. NixOS / devShell Check
    let nix_check = check_nixos_environment();
    if nix_check.is_nixos {
        if nix_check.missing_vars.is_empty() {
            println!("✅ NixOS environment: Ready (required build vars are set)");
        } else {
            println!("❌ NixOS environment: Missing required variables: {}", nix_check.missing_vars.join(", "));
            println!("   Hint: Enter the devShell via `nix-shell` or `direnv allow` to populate OpenSSL and pkg-config variables.");
            all_healthy = false;
        }
        
        if nix_check.dev_shell_active {
            println!("✅ Nix devShell: Active");
        } else {
            println!("⚠️ Nix devShell: Not active. Some tooling might fail if executed outside of shell.nix.");
        }
    }

    println!("=============================================");
    if all_healthy {
        println!("Verdict: Healthy! All environment components are fully operational.");
        std::process::exit(0);
    } else {
        println!("Verdict: Unhealthy! Some environment checks failed. Please review the ❌ items above.");
        std::process::exit(1);
    }
}
