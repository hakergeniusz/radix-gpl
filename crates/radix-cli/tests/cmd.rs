//! CLI-T006/007 (doc 09 §9): the `radix-cmd` REPL contract. XDG dirs are
//! redirected to a scratch dir; `refresh-interval=0` keeps startup strictly
//! cache-only (no network attempt, per doc 08 §5 refresh semantics).

use std::io::Write;
use std::process::{Command, Output, Stdio};

fn scratch() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("radix-cmd-tests-{}", std::process::id()));
    let settings = dir.join("radix");
    std::fs::create_dir_all(&settings).expect("create scratch dir");
    std::fs::write(
        settings.join("settings.ini"),
        "[settings]\nrefresh-interval=0\n",
    )
    .expect("write settings.ini");
    dir
}

fn radix_cmd(input: &[u8]) -> Output {
    let dir = scratch();
    let mut child = Command::new(env!("CARGO_BIN_EXE_radix-cmd"))
        .env("XDG_CONFIG_HOME", &dir)
        .env("XDG_DATA_HOME", &dir)
        .env("XDG_CACHE_HOME", &dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn radix-cmd");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(input)
        .expect("write stdin");
    child.wait_with_output().expect("wait radix-cmd")
}

#[test]
fn cli_t006_repl_solve_then_exit() {
    let out = radix_cmd(b"2+3\nexit\n");
    assert_eq!(out.stdout, b"> 5\n> ");
    assert!(
        out.stderr.is_empty(),
        "stderr: {:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn cli_t007_repl_eof_exits_cleanly() {
    let out = radix_cmd(b"2+3\n");
    assert_eq!(out.stdout, b"> 5\n> ");
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn cli_t007_empty_line_exits() {
    let out = radix_cmd(b"\n");
    assert_eq!(out.stdout, b"> ");
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn cmd_arg_expression() {
    // One-shot mode: `radix-cmd 2+3` prints the result without a prompt.
    let dir = scratch();
    let out = Command::new(env!("CARGO_BIN_EXE_radix-cmd"))
        .arg("2+3")
        .env("XDG_CONFIG_HOME", &dir)
        .env("XDG_DATA_HOME", &dir)
        .env("XDG_CACHE_HOME", &dir)
        .output()
        .expect("spawn radix-cmd");
    assert_eq!(out.stdout, b"5\n");
    assert_eq!(out.status.code(), Some(0));
}
