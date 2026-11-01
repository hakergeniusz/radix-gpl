//! CLI-T001..005 (doc 09 §9): the `radix` binary's headless contract,
//! byte-exact. Runs against the built binary; XDG dirs are redirected to a
//! scratch dir so the user's real settings/caches never leak in.

use std::process::{Command, Output};

fn scratch() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("radix-cli-tests-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn radix(args: &[&str]) -> Output {
    let dir = scratch();
    Command::new(env!("CARGO_BIN_EXE_radix"))
        .args(args)
        .env("XDG_CONFIG_HOME", &dir)
        .env("XDG_DATA_HOME", &dir)
        .env("XDG_CACHE_HOME", &dir)
        .output()
        .expect("spawn radix")
}

#[test]
fn cli_t001_solve_stdout_exit_zero() {
    let out = radix(&["--solve", "2+2"]);
    assert_eq!(out.stdout, b"4\n");
    assert!(
        out.stderr.is_empty(),
        "stderr: {:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn cli_t002_solve_division_by_zero() {
    let out = radix(&["--solve", "1/0"]);
    assert_eq!(out.stdout, b"");
    assert_eq!(out.stderr, b"Error: Division by zero is undefined\n");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn cli_t003_solve_invalid() {
    let out = radix(&["--solve", "1+"]);
    assert_eq!(out.stdout, b"");
    assert_eq!(out.stderr, b"Error: ErrorCode.INVALID\n");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn cli_t004_solve_in_hex() {
    let out = radix(&["--solve", "10 in hex"]);
    assert_eq!(out.stdout, "A₁₆\n".as_bytes());
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn cli_t005_version() {
    let out = radix(&["--version"]);
    assert_eq!(out.stdout, b"");
    let expected = format!("Radix {}\n", env!("CARGO_PKG_VERSION"));
    assert_eq!(out.stderr, expected.as_bytes());
    assert_eq!(out.status.code(), Some(0));
}
