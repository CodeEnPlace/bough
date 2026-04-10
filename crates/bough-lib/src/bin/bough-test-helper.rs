#![allow(clippy::zombie_processes)]

use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("flood-stdout") => flood(std::io::stdout(), &args[1..]),
        Some("flood-stderr") => flood(std::io::stderr(), &args[1..]),
        Some("spawn-and-wait") => spawn_and_wait(&args[1..]),
        Some("spawn-chain") => spawn_chain(&args[1..]),
        Some("sleep") => std::thread::sleep(std::time::Duration::from_secs(1000)),
        #[cfg(unix)]
        Some("spawn-own-pgroup") => spawn_own_pgroup(&args[1..]),
        _ => {
            eprintln!(
                "usage: bough-test-helper <flood-stdout|flood-stderr|spawn-and-wait|spawn-chain|spawn-own-pgroup|sleep> [args...]"
            );
            std::process::exit(1);
        }
    }
}

fn flood(mut w: impl Write, args: &[String]) {
    let bytes: usize = args
        .first()
        .and_then(|s| s.parse().ok())
        .unwrap_or(256 * 1024);
    let chunk = vec![b'x'; 1024];
    let mut written = 0;
    while written < bytes {
        let n = std::cmp::min(chunk.len(), bytes - written);
        w.write_all(&chunk[..n]).unwrap();
        written += n;
    }
    w.flush().unwrap();
}

fn spawn_chain(args: &[String]) {
    let pid_dir = args.first().expect("spawn-chain requires a pid-dir path");
    let depth: usize = args
        .get(1)
        .and_then(|s| s.parse().ok())
        .expect("spawn-chain requires a depth");

    let pid_file = std::path::Path::new(pid_dir).join(format!("depth-{depth}.pid"));
    std::fs::write(&pid_file, std::process::id().to_string()).unwrap();

    if depth > 0 {
        let _child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["spawn-chain", pid_dir, &(depth - 1).to_string()])
            .spawn()
            .expect("spawn child");
    }

    std::thread::sleep(std::time::Duration::from_secs(1000));
}

#[cfg(unix)]
fn spawn_own_pgroup(args: &[String]) {
    use std::os::unix::process::CommandExt;
    use std::sync::atomic::Ordering;

    let pid_dir = args
        .first()
        .expect("spawn-own-pgroup requires a pid-dir path");
    let pid_dir = std::path::Path::new(pid_dir);

    std::fs::write(pid_dir.join("parent.pid"), std::process::id().to_string()).unwrap();

    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["sleep"])
        .process_group(0)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn child in own pgroup");

    let child_pid = child.id() as i32;
    std::fs::write(pid_dir.join("child.pid"), child_pid.to_string()).unwrap();

    CHILD_PID_FOR_HANDLER.store(child_pid, Ordering::SeqCst);

    unsafe {
        libc::signal(
            libc::SIGTERM,
            handle_sigterm as *const () as libc::sighandler_t,
        );
    }

    let _ = child.wait();
    std::thread::sleep(std::time::Duration::from_secs(1000));
}

#[cfg(unix)]
static CHILD_PID_FOR_HANDLER: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

#[cfg(unix)]
extern "C" fn handle_sigterm(_sig: libc::c_int) {
    let child_pid = CHILD_PID_FOR_HANDLER.load(std::sync::atomic::Ordering::SeqCst);
    if child_pid > 0 {
        unsafe {
            libc::kill(-child_pid, libc::SIGKILL);
        }
    }
    unsafe {
        libc::_exit(0);
    }
}

fn spawn_and_wait(args: &[String]) {
    let pid_file = args
        .first()
        .expect("spawn-and-wait requires a pid-file path");
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["sleep"])
        .spawn()
        .expect("spawn child");
    std::fs::write(pid_file, child.id().to_string()).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1000));
}
