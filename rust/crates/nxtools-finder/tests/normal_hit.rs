use nxtools_extensions::PackageRegistry;
use nxtools_finder::{ApplicationRoot, ApplicationSource, FinderCatalog};

#[allow(unsafe_code)]
mod allocations {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    thread_local! {
        static ACTIVE: Cell<bool> = const { Cell::new(false) };
        static BYTES: Cell<usize> = const { Cell::new(0) };
    }

    struct MeasuredSystem;
    #[global_allocator]
    static ALLOCATOR: MeasuredSystem = MeasuredSystem;

    // SAFETY: every allocation/deallocation is forwarded unchanged to System.
    // Thread-local counters observe requested bytes only in the measured API call.
    unsafe impl GlobalAlloc for MeasuredSystem {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let _ = ACTIVE.try_with(|active| {
                if active.get() {
                    let _ = BYTES.try_with(|bytes| bytes.set(bytes.get() + layout.size()));
                }
            });
            // SAFETY: caller supplies the valid layout required by GlobalAlloc.
            unsafe { System.alloc(layout) }
        }
        unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
            // SAFETY: pointer/layout are passed unchanged to their original allocator.
            unsafe { System.dealloc(pointer, layout) }
        }
    }

    pub fn measure<T>(run: impl FnOnce() -> T) -> (T, usize) {
        BYTES.with(|bytes| bytes.set(0));
        ACTIVE.with(|active| active.set(true));
        let result = run();
        ACTIVE.with(|active| active.set(false));
        (result, BYTES.with(Cell::get))
    }
}

#[test]
fn local_normal_hit_does_not_allocate_a_discarded_ranked_catalog() {
    let temporary = tempfile::tempdir().expect("temporary catalog");
    let applications = temporary.path().join("applications");
    std::fs::create_dir(&applications).expect("catalog root");
    for index in 0..512 {
        std::fs::write(
            applications.join(format!("needle-{index:04}.lnk")),
            b"fixture",
        )
        .expect("synthetic shortcut");
    }
    let finder = FinderCatalog::open(
        temporary.path().join("history.db"),
        &[ApplicationRoot {
            path: applications,
            source: ApplicationSource::StartMenu,
        }],
    )
    .expect("catalog");
    let packages = PackageRegistry::default();
    let (results, ranked_bytes) = allocations::measure(|| {
        finder
            .search_with_packages_normal("needle", &[], &[], &packages, 200)
            .expect("complete ordinary results")
    });
    assert_eq!(results.len(), 200);
    let (hit, probe_bytes) = allocations::measure(|| {
        finder
            .has_local_normal_hit("needle", &[], &packages, 200)
            .expect("ordinary hit probe")
    });
    assert!(hit);
    println!(
        "LOCAL_NORMAL_HIT_FIXTURE entries=512 ranked_bytes={ranked_bytes} probe_bytes={probe_bytes}"
    );
    assert!(
        probe_bytes < ranked_bytes / 4,
        "existence probe must not build and rank the discarded result set: probe={probe_bytes}, ranked={ranked_bytes}"
    );
}

fn declarative_packages() -> PackageRegistry {
    let manifest = br#"{"schema_version":1,"id":"fixture.probe","name":"Probe","version":"1","handlers":[
        {"id":"web","title":"Web","feature":"quick_launch","entry":"command","keywords":["web","lookup"],"action":{"type":"website","url_template":"https://example.test/?q={query}"}},
        {"id":"run","title":"Run","feature":"quick_launch","entry":"command","keywords":["run"],"action":{"type":"program","executable":"C:\\Fixture\\tool.exe","argument_template":"--name %s"}},
        {"id":"places","title":"Places","feature":"quick_launch","entry":"command","keywords":["places"],"action":{"type":"items","items":[{"id":"docs","title":"Documentation","action":{"type":"folder","path":"C:\\Fixture\\Docs"}}]}}
    ]}"#;
    let mut packages = PackageRegistry::default();
    packages
        .install(
            nxtools_extensions::ImportPreview::from_json(manifest).expect("provider manifest"),
            "fixture".into(),
            false,
        )
        .expect("install");
    packages
        .set_enabled("fixture.probe", true, &[])
        .expect("enable");
    packages
}

#[test]
fn local_normal_hit_preserves_sources_package_state_and_final_rank() {
    let temporary = tempfile::tempdir().expect("temporary catalog");
    let applications = temporary.path().join("applications");
    std::fs::create_dir(&applications).expect("catalog root");
    for name in ["needle", "计算器", "年度报告"] {
        std::fs::write(applications.join(format!("{name}.lnk")), b"fixture").expect("shortcut");
    }
    let finder = FinderCatalog::open(
        temporary.path().join("history.db"),
        &[ApplicationRoot {
            path: applications,
            source: ApplicationSource::StartMenu,
        }],
    )
    .expect("catalog");
    let commands = [
        nxtools_core::CustomCommandSettings {
            keyword: "tool-needs-input".into(),
            program: r"C:\Fixture\tool.exe".into(),
            argument_template: "%s1".into(),
            ..Default::default()
        },
        nxtools_core::CustomCommandSettings {
            keyword: "tool-ready".into(),
            program: r"C:\Fixture\ready.exe".into(),
            ..Default::default()
        },
    ];
    let mut packages = declarative_packages();
    for enabled in [true, false] {
        packages
            .set_enabled("fixture.probe", enabled, &[])
            .expect("package state");
        for (query, expected) in [
            ("", false),
            ("   ", false),
            ("needle", true),
            ("jsq", true),
            ("bgao", true),
            ("年度报高", false),
            ("no-matching-provider", false),
            ("=1+1", true),
            ("=1+", false),
            ("tool", true),
            ("tool-ready", true),
            ("web hello", enabled),
            ("look", enabled),
            ("places doc", enabled),
            ("places missing", false),
            ("run hello", enabled),
        ] {
            let before = finder
                .search_with_packages_normal(query, &[], &commands, &packages, 200)
                .expect("complete normal result");
            assert_eq!(
                !before.is_empty(),
                expected,
                "fixture expectation: {query}, enabled={enabled}"
            );
            assert_eq!(
                finder
                    .has_local_normal_hit(query, &commands, &packages, 200)
                    .expect("probe"),
                expected,
                "{query}, enabled={enabled}"
            );
            assert_eq!(
                finder
                    .search_with_packages_normal(query, &[], &commands, &packages, 200)
                    .expect("unchanged final rank"),
                before
            );
            for limit in [0, 1, 2] {
                assert_eq!(
                    finder
                        .has_local_normal_hit(query, &commands, &packages, limit)
                        .expect("limited probe"),
                    !finder
                        .search_with_packages_normal(query, &[], &commands, &packages, limit)
                        .expect("limited complete search")
                        .is_empty(),
                    "{query}, limit={limit}"
                );
            }
        }
    }
    packages
        .set_enabled("fixture.probe", true, &[])
        .expect("enable");
    packages
        .set_aliases("fixture.probe", "web", vec!["lookup".into()], &[])
        .expect("alias");
    assert!(
        finder
            .has_local_normal_hit("lookup hello", &[], &packages, 200)
            .expect("current alias")
    );
    assert!(
        !finder
            .has_local_normal_hit("web hello", &[], &packages, 200)
            .expect("retired keyword")
    );
}

#[test]
fn local_normal_hit_preserves_environment_semantics() {
    if std::env::var_os("NXTOOLS_NORMAL_HIT_CHILD").is_none() {
        let output =
            std::process::Command::new(std::env::current_exe().expect("fixture executable"))
                .args([
                    "--exact",
                    "local_normal_hit_preserves_environment_semantics",
                    "--nocapture",
                ])
                .env("NXTOOLS_NORMAL_HIT_CHILD", "1")
                .env("NXTOOLS_NORMAL_HIT_VALUE", "owned fixture")
                .env("NXTOOLS_NORMAL_HIT_EMPTY", ";")
                .output()
                .expect("isolated environment fixture");
        assert!(
            output.status.success(),
            "environment fixture: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let temporary = tempfile::tempdir().expect("temporary catalog");
    let finder = FinderCatalog::open(temporary.path().join("history.db"), &[]).expect("catalog");
    let packages = PackageRegistry::default();
    for (query, expected) in [
        ("%NXTOOLS_NORMAL_HIT_VALUE%", true),
        ("%nxtools_normal_hit_value%", true),
        ("%NXTOOLS_NORMAL_HIT_EMPTY%", false),
    ] {
        assert_eq!(
            finder
                .has_local_normal_hit(query, &[], &packages, 200)
                .expect("environment probe"),
            expected,
            "{query}"
        );
        assert_eq!(
            !finder
                .search_with_packages_normal(query, &[], &[], &packages, 200)
                .expect("environment results")
                .is_empty(),
            expected
        );
    }
}
