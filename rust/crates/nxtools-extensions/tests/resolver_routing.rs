use nxtools_extensions::{ResolverCycleGuard, ResolverRoutingError};
#[test]
fn chain_cycle_guard_allows_pass_through_but_not_changed_url_revisit() {
    let original = url::Url::parse("https://fixture.test/page").expect("original");
    let mut guard = ResolverCycleGuard::new(&original);
    guard.accept(&original).expect("not-handled pass through");
    guard
        .accept(&url::Url::parse("https://fixture.test/file#fragment").expect("next"))
        .expect("next stage");
    guard
        .accept(&url::Url::parse("https://fixture.test/file#another").expect("same resource"))
        .expect("fragment isn't a resource change");
    assert_eq!(guard.accept(&original), Err(ResolverRoutingError::Cycle));
}
