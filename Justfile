target_dir := env("TARGET_DIR", "target")
dist_dir := env("DIST_DIR", "dist")
windows_target := "x86_64-pc-windows-gnu"
linux_target := "x86_64-unknown-linux-gnu"
mac_x86_target := "x86_64-apple-darwin"
mac_arm_target := "aarch64-apple-darwin"

name := `cargo metadata --format-version 1 --no-deps 2>/dev/null | jq -r '.packages[0].name'`
version := `cargo metadata --format-version 1 --no-deps 2>/dev/null | jq -r '.packages[0].version'`

set dotenv-load

default:
    @just --list

# development

dev *args:
    cargo run -- {{args}}

clean:
    cargo clean
    cargo clean --target-dir "{{target_dir}}"
    rm -rf {{dist_dir}}

check:
    cargo check --target {{windows_target}}
    cargo check --target {{linux_target}}
    cargo check --target {{mac_x86_target}}
    cargo check --target {{mac_arm_target}}

lint:
    cargo clippy --target {{windows_target}} -- -W clippy::pedantic
    cargo clippy --target {{linux_target}} -- -W clippy::pedantic
    cargo clippy --target {{mac_x86_target}} -- -W clippy::pedantic
    cargo clippy --target {{mac_arm_target}} -- -W clippy::pedantic

fmt:
    cargo fmt

test:
    cargo test

ci: check lint test
    cargo fmt -- --check
    @echo "All CI checks passed"

pre-commit: fmt lint test
    @echo "Ready to commit"

# builds - dev

build:
    cargo build

build-win: _ensure-dist
    cargo build --target {{windows_target}} --target-dir "{{target_dir}}"
    cp "{{target_dir}}/{{windows_target}}/debug/{{name}}.exe" "{{dist_dir}}/debug/{{name}}-windows-x86_64-debug.exe"

build-linux: _ensure-dist
    cargo build --target {{linux_target}} --target-dir "{{target_dir}}"
    cp "{{target_dir}}/{{linux_target}}/debug/{{name}}" "{{dist_dir}}/debug/{{name}}-linux-x86_64-debug"

build-mac-x86: _ensure-dist
    cargo build --target {{mac_x86_target}} --target-dir "{{target_dir}}"
    cp "{{target_dir}}/{{mac_x86_target}}/debug/{{name}}" "{{dist_dir}}/debug/{{name}}-mac-x86_64-debug"

build-mac-arm: _ensure-dist
    cargo build --target {{mac_arm_target}} --target-dir "{{target_dir}}"
    cp "{{target_dir}}/{{mac_arm_target}}/debug/{{name}}" "{{dist_dir}}/debug/{{name}}-mac-aarch64-debug"

build-mac: build-mac-x86 build-mac-arm

# builds - release

_ensure-dist:
    mkdir -p "{{dist_dir}}/debug"
    mkdir -p "{{dist_dir}}/release"

dist-win: _ensure-dist
    cargo build --target {{windows_target}} --target-dir "{{target_dir}}" --release
    cp "{{target_dir}}/{{windows_target}}/release/{{name}}.exe" "{{dist_dir}}/release/{{name}}-windows-x86_64.exe"
    @echo "Built: {{dist_dir}}/{{name}}-windows-x86_64.exe"

dist-linux: _ensure-dist
    cargo build --target {{linux_target}} --target-dir "{{target_dir}}" --release
    cp "{{target_dir}}/{{linux_target}}/release/{{name}}" "{{dist_dir}}/release/{{name}}-linux-x86_64"
    @echo "Built: {{dist_dir}}/{{name}}-linux-x86_64"

dist-mac-x86: _ensure-dist
    cargo build --target {{mac_x86_target}} --target-dir "{{target_dir}}" --release
    cp "{{target_dir}}/{{mac_x86_target}}/release/{{name}}" "{{dist_dir}}/release/{{name}}-mac-x86_64"
    @echo "Built: {{dist_dir}}/{{name}}-mac-x86_64"

dist-mac-arm: _ensure-dist
    cargo build --target {{mac_arm_target}} --target-dir "{{target_dir}}" --release
    cp "{{target_dir}}/{{mac_arm_target}}/release/{{name}}" "{{dist_dir}}/release/{{name}}-mac-aarch64"
    @echo "Built: {{dist_dir}}/{{name}}-mac-aarch64"

dist-mac: dist-mac-x86 dist-mac-arm

dist: dist-win dist-linux dist-mac
    cd "{{dist_dir}}/release" && sha256sum {{name}}-windows-x86_64.exe > {{name}}-windows-x86_64.exe.sha256
    cd "{{dist_dir}}/release" && sha256sum {{name}}-linux-x86_64 > {{name}}-linux-x86_64.sha256
    cd "{{dist_dir}}/release" && sha256sum {{name}}-mac-x86_64 > {{name}}-mac-x86_64.sha256
    cd "{{dist_dir}}/release" && sha256sum {{name}}-mac-aarch64 > {{name}}-mac-aarch64.sha256
    @echo "Generated checksums:"
    @echo "  {{dist_dir}}/release/{{name}}-windows-x86_64.exe.sha256"
    @echo "  {{dist_dir}}/release/{{name}}-linux-x86_64.sha256"
    @echo "  {{dist_dir}}/release/{{name}}-mac-x86_64.sha256"
    @echo "  {{dist_dir}}/release/{{name}}-mac-aarch64.sha256"
    @echo "Release builds completed in {{dist_dir}}/release"