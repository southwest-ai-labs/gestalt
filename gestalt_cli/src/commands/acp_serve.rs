use tracing::{error, info};

pub async fn serve(stdio: bool, port: Option<u16>, shutdown: &crate::shutdown::ShutdownCoordinator) -> anyhow::Result<()> {
    let transport = if stdio { "stdio" } else { "http" };
    let port = port.unwrap_or(8082);
    
    info!("Starting ACP Server...");
    println!("🚀 Starting Standalone ACP Server ({})", transport);

    let mut cmd = tokio::process::Command::new("gestalt_acp");
    cmd.arg("--transport").arg(transport);
    if !stdio {
        cmd.arg("--port").arg(port.to_string());
    }

    let mut child = cmd.spawn()?;
    
    match crate::wait_for_child_or_shutdown(&mut child, shutdown, "standalone ACP server").await? {
        Some(status) => {
            if !status.success() {
                error!("ACP server exited with error");
            }
        },
        None => {
            shutdown.record_cancelled(1);
        }
    }
    Ok(())
}
