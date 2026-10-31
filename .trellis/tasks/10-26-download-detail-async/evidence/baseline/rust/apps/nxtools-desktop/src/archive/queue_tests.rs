use super::*;
fn resource(source: &str, output: &str) -> Resources {
    Resources {
        sources: vec![source.into()],
        output: Some(output.into()),
        memory: 832 << 20,
    }
}
#[test]
fn fifo_capacity_duplicate_and_queued_cancel_are_bounded() {
    let mut queue = JobQueue::default();
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();
    queue.enqueue(first, resource("a", "out-a")).expect("first");
    assert_eq!(queue.next(), Some(first));
    queue
        .enqueue(second, resource("b", "out-b"))
        .expect("second");
    assert_eq!(queue.next(), Some(second));
    let ids = (0..32).map(|_| Uuid::new_v4()).collect::<Vec<_>>();
    for (index, id) in ids.iter().enumerate() {
        queue
            .enqueue(*id, resource(&format!("s{index}"), &format!("o{index}")))
            .expect("finite queue");
    }
    assert_eq!(
        queue.enqueue(Uuid::new_v4(), resource("overflow", "overflow")),
        Err(AdmissionError::Full)
    );
    assert_eq!(
        queue.enqueue(first, resource("duplicate", "duplicate")),
        Err(AdmissionError::Duplicate)
    );
    assert!(queue.cancel_queued(ids[0]));
    assert!(!queue.cancel_queued(first));
    assert_eq!(queue.next(), None);
    assert!(queue.finish(first, true));
    assert_eq!(queue.next(), Some(ids[1]));
}
#[test]
fn blocked_head_is_not_bypassed_and_cleanup_incomplete_retains_slot() {
    let mut queue = JobQueue::default();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    queue
        .enqueue(a, resource(r"C:\source.zip", r"C:\out"))
        .expect("a");
    assert_eq!(queue.next(), Some(a));
    queue
        .enqueue(b, resource(r"c:/SOURCE.zip", r"C:\other"))
        .expect("b");
    queue
        .enqueue(c, resource("independent", "elsewhere"))
        .expect("c");
    assert_eq!(queue.next(), None, "strict FIFO cannot bypass blocked head");
    assert!(!queue.finish(a, false));
    assert_eq!(queue.next(), None);
    assert!(queue.finish(a, true));
    assert_eq!(queue.next(), Some(b));
    assert_eq!(queue.next(), Some(c));
}
#[test]
fn overlapping_outputs_and_aggregate_memory_do_not_run_unordered() {
    let mut queue = JobQueue::default();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    queue.enqueue(a, resource("a", r"C:\out")).expect("a");
    assert_eq!(queue.next(), Some(a));
    queue.enqueue(b, resource("b", r"c:/out/child")).expect("b");
    assert_eq!(queue.next(), None);
    assert!(queue.finish(a, true));
    assert_eq!(queue.next(), Some(b));
    assert_eq!(
        queue.enqueue(
            Uuid::new_v4(),
            Resources {
                sources: vec!["c".into()],
                output: None,
                memory: 2 << 30
            }
        ),
        Err(AdmissionError::Memory)
    );
}
