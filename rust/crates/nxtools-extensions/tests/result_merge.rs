use nxtools_extensions::{ExtensionEffect, InvocationIdentity, ProviderItem, ResultMerge};
use uuid::Uuid;
fn identity(package: &str) -> InvocationIdentity {
    InvocationIdentity {
        run_id: Uuid::new_v4(),
        correlation_id: Uuid::new_v4(),
        generation: 9,
        package_id: package.into(),
        handler_id: "provider".into(),
        content_hash: "a".repeat(64),
    }
}
fn item(id: &str, title: &str) -> ProviderItem {
    ProviderItem {
        id: id.into(),
        title: title.into(),
        subtitle: String::new(),
        action: ExtensionEffect::Copy { text: title.into() },
    }
}
#[test]
fn late_results_update_in_place_keep_selection_and_reject_stale_content() {
    let mut merge = ResultMerge::new(9, false);
    merge.set_normal(vec![
        ("normal-first".into(), "first".to_string()),
        ("normal-second".into(), "second".into()),
    ]);
    assert!(merge.select("normal-first"));
    let identity = identity("fixture.one");
    assert!(merge.register(identity.clone()));
    assert!(
        merge.apply(&identity, false, &[item("one", "original")], |item, _| item
            .title
            .clone())
    );
    assert_eq!(merge.selected_id(), Some("normal-first"));
    assert_eq!(merge.rows()[0].id, "normal-first");
    let plugin_id = merge.rows()[2].id.clone();
    merge.select(&plugin_id);
    merge.apply(
        &identity,
        false,
        &[item("one", "updated"), item("two", "new")],
        |item, _| item.title.clone(),
    );
    assert_eq!(merge.rows()[2].value, "updated");
    assert_eq!(merge.selected_id(), Some(plugin_id.as_str()));
    let mut stale = identity.clone();
    stale.content_hash = "b".repeat(64);
    assert!(!merge.apply(&stale, true, &[], |item, _| item.title.clone()));
    assert_eq!(merge.rows().len(), 4);
    merge.apply(&identity, true, &[item("two", "retained")], |item, _| {
        item.title.clone()
    });
    assert_eq!(
        merge.selected_id(),
        Some("extension:fixture.one/provider/two")
    );
    merge.retire("fixture.one");
    assert_eq!(merge.rows().len(), 2);
    assert_eq!(merge.selected_id(), Some("normal-second"));
    assert!(
        !merge.apply(&identity, false, &[item("one", "late")], |item, _| item
            .title
            .clone())
    );
}
#[test]
fn quotas_bound_accumulated_batches_and_normal_refresh_preserves_plugin_identity() {
    let mut merge = ResultMerge::new(9, false);
    merge.set_normal(vec![("normal".into(), "normal".to_string())]);
    for index in 0..6 {
        let identity = identity(&format!("fixture.{index}"));
        merge.register(identity.clone());
        let items = (0..6)
            .map(|index| item(&format!("item-{index}"), "value"))
            .collect::<Vec<_>>();
        merge.apply(&identity, false, &items, |item, _| item.title.clone());
    }
    assert_eq!(merge.rows().len(), 13);
    assert_eq!(merge.selected_id(), Some("normal"));
    merge.set_normal(vec![("normal".into(), "changed".into())]);
    assert_eq!(merge.rows()[0].value, "changed");
    assert_eq!(merge.rows().len(), 13);
    let mut keyword = ResultMerge::new(9, true);
    let identity = identity("fixture.keyword");
    assert!(keyword.register(identity.clone()));
    assert!(!keyword.register(self_identity()));
    let items = (0..250)
        .map(|index| item(&format!("item-{index}"), "value"))
        .collect::<Vec<_>>();
    keyword.apply(&identity, true, &items, |item, _| item.title.clone());
    assert_eq!(keyword.rows().len(), 200);
}
fn self_identity() -> InvocationIdentity {
    identity("fixture.other")
}
