#!/usr/bin/env pwsh
# Gestalt Git Hooks Installer
# Run this script to install Gestalt git hooks.

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
Set-Location $repoRoot

$gitHooksDir = Join-Path $repoRoot ".git/hooks"

if (-not (Test-Path $gitHooksDir)) {
    if (-not (Test-Path (Join-Path $repoRoot ".git"))) {
        Write-Error "Error: .git directory not found in $repoRoot"
        exit 1
    }
    Write-Host "Creating .git/hooks directory..."
    New-Item -ItemType Directory -Path $gitHooksDir -Force | Out-Null
}

Write-Host "Installing Gestalt Git hooks..."

# Install pre-commit hook
if (Test-Path "hooks/pre-commit.sh") {
    Copy-Item -Path "hooks/pre-commit.sh" -Destination (Join-Path $gitHooksDir "pre-commit") -Force
} elseif (Test-Path "hooks/pre-commit") {
    Copy-Item -Path "hooks/pre-commit" -Destination (Join-Path $gitHooksDir "pre-commit") -Force
}

# Install pre-push hook
if (Test-Path "hooks/pre-push.sh") {
    Copy-Item -Path "hooks/pre-push.sh" -Destination (Join-Path $gitHooksDir "pre-push") -Force
}

# Install post-commit hook
if (Test-Path "hooks/post-commit.sh") {
    Copy-Item -Path "hooks/post-commit.sh" -Destination (Join-Path $gitHooksDir "post-commit") -Force
}

Write-Host "✅ Hooks installed successfully!"
Write-Host ""
Write-Host "To run validation manually:"
Write-Host "  cargo fmt --check        (format check)"
Write-Host "  cargo clippy --all       (lint check)"
Write-Host "  cargo check --workspace  (build check)"
