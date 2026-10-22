use nxtools_extensions::{
    ImportPreview, PackageError, PackageRegistry, SecretFuture, SecretStore, SecretStoreError,
    SettingsDraft, SettingsSaveError, save_settings,
};
use std::{collections::BTreeMap, sync::Mutex};

#[derive(Default)]
struct Store {
    state: Mutex<State>,
}
#[derive(Default)]
struct State {
    values: BTreeMap<String, String>,
    writes: usize,
    fail_after_mutation: Vec<usize>,
}
impl SecretStore for Store {
    fn get<'a>(&'a self, _package: &'a str, key: &'a str) -> SecretFuture<'a, Option<String>> {
        Box::pin(async move { Ok(self.state.lock().expect("store").values.get(key).cloned()) })
    }
    fn set<'a>(
        &'a self,
        _package: &'a str,
        key: &'a str,
        value: Option<&'a str>,
    ) -> SecretFuture<'a, ()> {
        Box::pin(async move {
            let mut state = self.state.lock().expect("store");
            state.writes += 1;
            match value {
                Some(value) => {
                    state.values.insert(key.into(), value.into());
                }
                None => {
                    state.values.remove(key);
                }
            }
            if state.fail_after_mutation.contains(&state.writes) {
                return Err(SecretStoreError);
            }
            Ok(())
        })
    }
}
fn package() -> nxtools_extensions::InstalledPackage {
    let manifest = br#"{"schema_version":1,"id":"fixture.settings","name":"Settings","version":"1","settings":[{"key":"name","title":"Name","type":"text","default":"initial"},{"key":"first","title":"First","type":"password"},{"key":"second","title":"Second","type":"password"}],"handlers":[{"id":"web","title":"Web","feature":"quick_launch","entry":"command","keywords":["web"],"action":{"type":"website","url_template":"https://fixture.test"}}]}"#;
    let mut registry = PackageRegistry::default();
    registry
        .install(
            ImportPreview::from_json(manifest).expect("preview"),
            "fixture".into(),
            false,
        )
        .expect("install");
    registry.packages.remove(0)
}
fn draft() -> SettingsDraft {
    SettingsDraft {
        values: BTreeMap::from([("name".into(), serde_json::json!("saved"))]),
        secrets: BTreeMap::from([
            ("first".into(), Some("new-first".into())),
            ("second".into(), Some("new-second".into())),
        ]),
    }
}

#[tokio::test(flavor = "current_thread")]
async fn attempted_secret_mutation_is_compensated_at_every_write_and_config_failure() {
    for fail in [1usize, 2, 0] {
        let store = Store::default();
        {
            let mut state = store.state.lock().expect("store");
            state.values.insert("first".into(), "prior-first".into());
            if fail != 0 {
                state.fail_after_mutation.push(fail);
            }
        }
        let result: Result<(), _> = save_settings(&package(), draft(), &store, |_| async {
            Err(PackageError::FileOperation)
        })
        .await;
        assert_eq!(result, Err(SettingsSaveError::SaveFailed));
        let state = store.state.lock().expect("store");
        assert_eq!(
            state.values,
            BTreeMap::from([("first".into(), "prior-first".into())]),
            "all attempted mutations including failing write are restored"
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn incomplete_compensation_is_truthful_and_invalid_or_cancelled_draft_has_no_writes() {
    let store = Store::default();
    store.state.lock().expect("store").fail_after_mutation = vec![2, 3];
    let result: Result<(), _> =
        save_settings(&package(), draft(), &store, |_| async { Ok(()) }).await;
    assert_eq!(result, Err(SettingsSaveError::CompensationFailed));
    let store = Store::default();
    let cancelled = draft();
    drop(cancelled);
    assert_eq!(store.state.lock().expect("store").writes, 0);
    let mut invalid = draft();
    invalid
        .values
        .insert("first".into(), serde_json::json!("secret"));
    let result: Result<(), _> =
        save_settings(&package(), invalid, &store, |_| async { Ok(()) }).await;
    assert_eq!(result, Err(SettingsSaveError::Invalid));
    assert_eq!(store.state.lock().expect("store").writes, 0);
}

#[tokio::test(flavor = "current_thread")]
async fn successful_save_commits_only_nonsecret_values_after_credentials() {
    let store = Store::default();
    let values = save_settings(&package(), draft(), &store, |values| async { Ok(values) })
        .await
        .expect("save");
    assert_eq!(
        values,
        BTreeMap::from([("name".into(), serde_json::json!("saved"))])
    );
    assert_eq!(store.state.lock().expect("store").writes, 2);
}
