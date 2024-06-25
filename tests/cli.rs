use assert_cmd::assert::OutputAssertExt;
use assert_cmd::cargo::CommandCargoExt;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time;

// Arbitrarily picked for system this was coded on
const RACE_AVOIDING_SLEEP: time::Duration = time::Duration::from_millis(100);

fn child_proc(args: Vec<&str>) -> Child {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    return cmd.spawn().unwrap();
}

#[test]
fn passes_through_input() {
    let mut child = child_proc(vec![]);
    let mut stdin = child.stdin.take().unwrap();

    stdin.write_all(b"test\n").unwrap();
    stdin.flush().unwrap();
    std::mem::drop(stdin);

    let output = child.wait_with_output().unwrap();
    output.assert().success().stdout("test\n");
}

#[test]
fn adds_signal_strings() {
    let mut child = child_proc(vec![]);
    let pid = Pid::from_raw(child.id().try_into().unwrap());
    let mut stdin = child.stdin.take().unwrap();

    stdin.write_all(b"start\n").unwrap();
    stdin.flush().unwrap();
    // Sleep so that subprocess can setup signal handlers instead of running into
    // SIGUSR* signals default handlers (terminate)
    thread::sleep(RACE_AVOIDING_SLEEP);
    signal::kill(pid, Signal::SIGUSR2).unwrap();
    thread::sleep(RACE_AVOIDING_SLEEP);
    signal::kill(pid, Signal::SIGUSR1).unwrap();
    thread::sleep(RACE_AVOIDING_SLEEP);
    stdin.write_all(b"end\n").unwrap();
    stdin.flush().unwrap();
    std::mem::drop(stdin);

    let output = child.wait_with_output().unwrap();
    output
        .assert()
        .success()
        .stdout("start\n===USR2===\n===USR1===\nend\n");
}

#[test]
fn starting_in_discard_waits_for_signal() {
    let mut child = child_proc(vec!["-d"]);
    let pid = Pid::from_raw(child.id().try_into().unwrap());
    let mut stdin = child.stdin.take().unwrap();

    stdin.write_all(b"ignore\n").unwrap();
    stdin.flush().unwrap();
    // Sleep so that subprocess can setup signal handlers instead of running into
    // SIGUSR* signals default handlers (terminate)
    thread::sleep(RACE_AVOIDING_SLEEP);
    signal::kill(pid, Signal::SIGUSR2).unwrap();
    thread::sleep(RACE_AVOIDING_SLEEP);
    signal::kill(pid, Signal::SIGUSR1).unwrap();
    thread::sleep(RACE_AVOIDING_SLEEP);
    stdin.write_all(b"test\n").unwrap();
    stdin.flush().unwrap();
    std::mem::drop(stdin);

    let output = child.wait_with_output().unwrap();
    output.assert().success().stdout("===USR1===\ntest\n");
}
