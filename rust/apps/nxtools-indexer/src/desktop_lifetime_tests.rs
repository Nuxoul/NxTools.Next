use super::*;
use std::process::{Child, Command, Stdio};
use std::thread;

fn owner() -> Child {
    Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Start-Sleep -Seconds 30",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("owned fixture process")
}

fn end(child: &mut Child) {
    child.kill().expect("terminate owned fixture");
    child.wait().expect("reap owned fixture");
}

fn until_stopped(owners: &mut DesktopLifetime) -> StopReason {
    for _ in 0..100 {
        if let Some(reason) = owners.poll(Instant::now()).expect("inspect owner handle") {
            return reason;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("owner process handle did not signal exit");
}

#[test]
fn initial_no_owner_deadline_and_unadmitted_clients_do_not_extend_it() {
    let start = Instant::now();
    let mut owners = DesktopLifetime::new(start, Duration::from_secs(15));
    let mut unadmitted = owner();
    assert_eq!(
        owners.poll(start + Duration::from_secs(14)).expect("poll"),
        None
    );
    assert_eq!(
        owners.poll(start + Duration::from_secs(15)).expect("poll"),
        Some(StopReason::NoOwner)
    );
    end(&mut unadmitted);
}

#[test]
fn authenticated_owner_survives_pipe_disconnect_and_duplicate_hello() {
    let mut child = owner();
    let mut owners = DesktopLifetime::new(Instant::now(), Duration::from_secs(15));
    assert!(owners.admit(child.id()).expect("open process handle"));
    assert!(!owners.admit(child.id()).expect("duplicate pid"));
    // No pipe is held: the retained process object owns service life.
    assert_eq!(
        owners
            .poll(Instant::now() + Duration::from_secs(30))
            .expect("poll"),
        None
    );
    end(&mut child);
    assert_eq!(until_stopped(&mut owners), StopReason::LastOwnerExited);
}

#[test]
fn one_of_two_owners_exiting_does_not_stop_the_service() {
    let mut first = owner();
    let mut second = owner();
    let mut owners = DesktopLifetime::new(Instant::now(), Duration::from_secs(15));
    assert!(owners.admit(first.id()).expect("first owner"));
    assert!(owners.admit(second.id()).expect("second owner"));
    end(&mut first);
    assert_eq!(owners.poll(Instant::now()).expect("poll"), None);
    assert_eq!(owners.count(), 1);
    end(&mut second);
    assert_eq!(until_stopped(&mut owners), StopReason::LastOwnerExited);
}

#[test]
fn explicit_stop_releases_all_process_handles() {
    let mut child = owner();
    let mut owners = DesktopLifetime::new(Instant::now(), Duration::from_secs(15));
    assert!(owners.admit(child.id()).expect("owner"));
    owners.clear();
    assert_eq!(owners.count(), 0);
    end(&mut child);
}
