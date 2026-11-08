//! End-to-End Test for Multi-Agent Conflict Resolution
//!
//! This test verifies that the `gestalt_router` logic exposed through the CLI effectively
//! detects and handles conflicts between multiple synthetic agents modifying overlapping files.
//!
//! describe: "Multi-Agent Conflict Resolution E2E Integration Test"
//! it("should detect conflicts and reject unmergeable branches")

use std::process::Command;
use tempfile::TempDir;

#[tokio::test]
async fn test_it_detects_conflicts_and_rolls_back() {
    let tmp = TempDir::new().unwrap();
    let repo_path = tmp.path().join("repo");
    std::fs::create_dir_all(&repo_path).unwrap();

    // Init git repo
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(&repo_path)
        .status()
        .unwrap();

    // Create a base file
    std::fs::write(repo_path.join("file.txt"), "base content\n").unwrap();
    Command::new("git")
        .args(["add", "file.txt"])
        .current_dir(&repo_path)
        .status()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(&repo_path)
        .status()
        .unwrap();

    let run_id = uuid::Uuid::new_v4();

    // Create synthetic branch 1
    Command::new("git")
        .args(["checkout", "-b", "agent_1_branch"])
        .current_dir(&repo_path)
        .status()
        .unwrap();
    std::fs::write(repo_path.join("file.txt"), "agent 1 content\n").unwrap();
    Command::new("git")
        .args(["commit", "-am", "agent 1 commit"])
        .current_dir(&repo_path)
        .status()
        .unwrap();
    let sha1 = String::from_utf8_lossy(
        &Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&repo_path)
            .output()
            .unwrap()
            .stdout,
    ).trim().to_string();

    // Create synthetic branch 2
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(&repo_path)
        .status()
        .unwrap();
    Command::new("git")
        .args(["checkout", "-b", "agent_2_branch"])
        .current_dir(&repo_path)
        .status()
        .unwrap();
    std::fs::write(repo_path.join("file.txt"), "agent 2 content\n").unwrap();
    Command::new("git")
        .args(["commit", "-am", "agent 2 commit"])
        .current_dir(&repo_path)
        .status()
        .unwrap();
    let sha2 = String::from_utf8_lossy(
        &Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&repo_path)
            .output()
            .unwrap()
            .stdout,
    ).trim().to_string();
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(&repo_path)
        .status()
        .unwrap();

    let integrate_branches = vec![
        ("agent-1".to_string(), sha1),
        ("agent-2".to_string(), sha2),
    ];

    let base_sha = String::from_utf8_lossy(
        &Command::new("git")
            .args(["rev-parse", "main"])
            .current_dir(&repo_path)
            .output()
            .unwrap()
            .stdout,
    ).trim().to_string();

    let result = gestalt_router::integrate::integrate_branches(
        &repo_path,
        &base_sha,
        &format!("integration-{}", run_id),
        &integrate_branches,
    ).await.unwrap();

    assert!(result.merge_sha.is_empty(), "Should not successfully merge due to conflict");
    assert!(!result.conflicts.is_empty(), "Should detect conflicts");
    
    // Check if worktree is dirty
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&repo_path)
        .output()
        .unwrap();
    assert!(status.stdout.is_empty(), "Worktree should be clean after failed merge and rollback");
}
