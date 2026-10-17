@echo off
rem Build (on first run and after plugin updates) and launch the Smartpowers progress board.
setlocal
set "SKILL_ROOT=%~dp0.."
set "MANIFEST=%SKILL_ROOT%\board\Cargo.toml"
set "BIN=%SKILL_ROOT%\board\target\release\smartpowers-board.exe"

where cargo >nul 2>nul
if errorlevel 1 (
  echo The Smartpowers board is built from source and needs the Rust toolchain. 1>&2
  echo Install Rust from https://rustup.rs, then run this command again. 1>&2
  exit /b 1
)

if not exist "%BIN%" echo Building the Smartpowers board ^(first run only^)... 1>&2
rem A no-op when the binary is already up to date with the source.
cargo build --release --quiet --manifest-path "%MANIFEST%"
if errorlevel 1 exit /b 1

"%BIN%" %*
