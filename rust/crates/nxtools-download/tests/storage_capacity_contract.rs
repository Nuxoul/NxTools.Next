use nxtools_download::{DownloadStorageCapacityError, DownloadStorageRequirement};

const MIB: u64 = 1024 * 1024;

#[test]
fn capacity_requirement_uses_durable_progress_and_a_bounded_reserve() {
    let small = DownloadStorageRequirement::for_transfer(100 * MIB, 25 * MIB)
        .expect("small transfer requirement");
    assert_eq!(small.remaining_bytes(), 75 * MIB);
    assert_eq!(small.reserve_bytes(), 64 * MIB);
    assert_eq!(small.required_bytes(), 139 * MIB);

    let large = DownloadStorageRequirement::for_transfer(20 * 1024 * MIB, 2 * 1024 * MIB)
        .expect("large transfer requirement");
    assert_eq!(large.remaining_bytes(), 18 * 1024 * MIB);
    assert_eq!(large.reserve_bytes(), 512 * MIB);
    assert_eq!(large.required_bytes(), 18 * 1024 * MIB + 512 * MIB);
}

#[test]
fn capacity_boundary_accepts_exact_space_and_reports_one_byte_short() {
    let requirement =
        DownloadStorageRequirement::for_transfer(512 * MIB, 128 * MIB).expect("requirement");

    requirement
        .ensure_available(requirement.required_bytes())
        .expect("exact capacity is sufficient");
    let error = requirement
        .ensure_available(requirement.required_bytes() - 1)
        .expect_err("one byte short is insufficient");

    assert_eq!(
        error,
        DownloadStorageCapacityError::Insufficient {
            available_bytes: requirement.required_bytes() - 1,
            required_bytes: requirement.required_bytes(),
        }
    );
}

#[test]
fn completed_transfer_needs_no_reserve_and_progress_is_clamped() {
    let complete =
        DownloadStorageRequirement::for_transfer(64 * MIB, 80 * MIB).expect("complete transfer");

    assert_eq!(complete.remaining_bytes(), 0);
    assert_eq!(complete.reserve_bytes(), 0);
    assert_eq!(complete.required_bytes(), 0);
}
