$ErrorActionPreference = "Stop"

function Require-Command($Name) {
  if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
    throw "Missing required command: $Name"
  }
}

Require-Command flutter
Require-Command dart
Require-Command rustup
Require-Command cargo

Write-Host "== Flutter =="
flutter --version

Write-Host "== Dart =="
dart --version

Write-Host "== Rust host =="
rustup run stable rustc -vV

$hostLine = rustup run stable rustc -vV | Select-String "^host:"
if ($hostLine -and $hostLine.ToString() -notmatch "x86_64-pc-windows-gnu") {
  Write-Warning "Rust stable host is not x86_64-pc-windows-gnu. Android FRB builds may fail on Windows."
  Write-Warning "Run: rustup set default-host x86_64-pc-windows-gnu"
}

Write-Host "== Rust targets =="
rustup target list --installed

$targets = rustup target list --installed
foreach ($target in @("aarch64-linux-android", "x86_64-linux-android")) {
  if ($targets -notcontains $target) {
    Write-Warning "Missing Rust target: $target"
    Write-Warning "Run: rustup target add $target --toolchain stable"
  }
}

Write-Host "== Rust components =="
rustup component list --installed | Select-String "rustfmt|clippy"

Write-Host "Doctor completed."