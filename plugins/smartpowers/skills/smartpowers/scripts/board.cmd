@echo off
rem Build (on first run and after plugin updates) and launch the Smartpowers progress board.
rem
rem Uses cargo from PATH or %USERPROFILE%\.cargo\bin. If neither exists, installs a
rem private Rust toolchain with rustup into %SMARTPOWERS_RUST_HOME%
rem (default: %LOCALAPPDATA%\smartpowers\rust) without changing the user's PATH or
rem %USERPROFILE%\.cargo. The private toolchain targets windows-gnu, which ships
rem its own linker, so Visual Studio Build Tools are not required.
setlocal
set "SKILL_ROOT=%~dp0.."
set "MANIFEST=%SKILL_ROOT%\board\Cargo.toml"
set "BIN=%SKILL_ROOT%\board\target\release\smartpowers-board.exe"

where cargo >nul 2>nul && goto build
if exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
  set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
  goto build
)
call :private_toolchain || exit /b 1

:build
if not exist "%BIN%" echo Building the Smartpowers board ^(first run only^)... 1>&2
rem A no-op when the binary is already up to date with the source.
cargo build --release --quiet --manifest-path "%MANIFEST%"
if errorlevel 1 exit /b 1
"%BIN%" %*
exit /b %errorlevel%

:private_toolchain
if defined SMARTPOWERS_RUST_HOME (set "RUST_HOME=%SMARTPOWERS_RUST_HOME%") else (set "RUST_HOME=%LOCALAPPDATA%\smartpowers\rust")
set "RUSTUP_HOME=%RUST_HOME%\rustup"
set "CARGO_HOME=%RUST_HOME%\cargo"
set "PATH=%CARGO_HOME%\bin;%PATH%"
if exist "%CARGO_HOME%\bin\cargo.exe" exit /b 0

echo cargo not found; installing a private Rust toolchain into %RUST_HOME% ^(one time^)... 1>&2
set "INSTALLER=%TEMP%\smartpowers-rustup-init-%RANDOM%.exe"
powershell -NoProfile -ExecutionPolicy Bypass -Command "$ProgressPreference='SilentlyContinue'; Invoke-WebRequest -UseBasicParsing -Uri 'https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe' -OutFile $env:INSTALLER"
if errorlevel 1 (
  echo Could not download rustup. Install Rust from https://rustup.rs and try again. 1>&2
  exit /b 1
)
"%INSTALLER%" -y -q --no-modify-path --profile minimal --default-toolchain stable --default-host x86_64-pc-windows-gnu 1>&2
set "STATUS=%errorlevel%"
del "%INSTALLER%" >nul 2>nul
if not "%STATUS%"=="0" echo Rust toolchain installation failed. Install Rust from https://rustup.rs and try again. 1>&2
exit /b %STATUS%
