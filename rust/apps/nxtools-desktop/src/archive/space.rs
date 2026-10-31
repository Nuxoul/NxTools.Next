//! Bounded advisory allocation from the original browser snapshot. Never IO or admission.
use nxtools_protocol::{
    archive_browser::ArchiveBrowserContext, archive_job::*, archive_read::ArchiveReadEntry,
    archive_space::ArchiveSpaceObservation,
};
use std::collections::HashSet;

#[derive(Clone)]
pub(super) struct Metadata {
    pub browser: ArchiveBrowserContext,
    pub entries: Vec<ArchiveReadEntry>,
}
#[derive(Clone, Default)]
pub(super) struct Estimate {
    pub payload: Option<u64>,
    pub allocation: Option<u64>,
    pub lower_bound: u64,
    pub limitation: Option<&'static str>,
}
#[derive(Clone)]
pub(super) struct Plan {
    pub observation: ArchiveSpaceObservation,
    pub estimate: Estimate,
    pub concurrent_jobs: usize,
}
impl Plan {
    pub fn remaining(&self) -> Option<u64> {
        Some(
            self.observation
                .available_bytes
                .saturating_sub(self.estimate.allocation?),
        )
    }
    pub fn shortfall(&self) -> Option<u64> {
        Some(
            self.estimate
                .allocation?
                .saturating_sub(self.observation.available_bytes),
        )
    }
}
fn round(bytes: u64, unit: u64) -> Option<u64> {
    if bytes == 0 {
        return Some(0);
    }
    bytes
        .checked_add(unit.checked_sub(1)?)?
        .checked_div(unit)?
        .checked_mul(unit)
}
/// Add only held-native target-directory allowance to the original member estimate.
pub(super) fn estimate_observed(
    request: &ArchiveJobRequest,
    metadata: Option<&Metadata>,
    observation: &ArchiveSpaceObservation,
) -> Estimate {
    let mut estimate = estimate(request, metadata, observation.allocation_bytes);
    let count = match observation.missing_destination_directories {
        Some(count) if count <= 64 && (count != 0) == observation.destination_missing => {
            Some(count)
        }
        None if !observation.destination_missing => Some(0),
        _ => None,
    };
    let Some(count) = count else {
        estimate.allocation = None;
        estimate.limitation = Some("缺失目标目录数量未知或观察不一致，无法准确估算新增空间");
        return estimate;
    };
    // The original empty-archive execution does not create a destination.
    if metadata.is_some_and(|metadata| metadata.entries.is_empty()) {
        return estimate;
    }
    if let Some(allocation) = estimate.allocation {
        estimate.allocation = u64::from(count)
            .checked_mul(observation.allocation_bytes)
            .and_then(|extra| allocation.checked_add(extra));
        if estimate.allocation.is_none() {
            estimate.limitation = Some("目标目录空间余量计算溢出，无法准确估算");
        }
    }
    estimate
}

pub(super) fn estimate(
    request: &ArchiveJobRequest,
    metadata: Option<&Metadata>,
    unit: u64,
) -> Estimate {
    let unknown = |reason| Estimate {
        limitation: Some(reason),
        ..Default::default()
    };
    if !unit.is_power_of_two() || unit > 16 << 20 {
        return unknown("目标卷分配粒度未知");
    }
    let Some(metadata) = metadata else {
        return unknown("没有匹配的完整浏览快照；可先打开压缩包再重新检查，不为预检重复扫描归档");
    };
    if metadata.entries.len() > 65_536 {
        return unknown("成员数超过原解压计划的有界估算范围");
    }
    let Some(extract) = request.extract.as_ref() else {
        return unknown("不是解压请求");
    };
    if matches!(
        metadata.browser.format.as_str(),
        "TarGz" | "TarBz2" | "TarXz" | "TarZstd"
    ) {
        return unknown(
            "压缩 TAR 的原流程还有临时卷 spool；当前无法准确估算全流程峰值，不提前扣除或回收来源",
        );
    }
    let ordinals = extract
        .selection
        .as_ref()
        .map(|selection| selection.ordinals.iter().copied().collect::<HashSet<_>>());
    if extract
        .selection
        .as_ref()
        .is_some_and(|selection| selection.browser != metadata.browser)
    {
        return unknown("源快照已变化，请重新选择成员");
    }
    let prefixes = metadata
        .entries
        .iter()
        .filter(|entry| {
            entry.kind == "Directory"
                && ordinals
                    .as_ref()
                    .is_some_and(|ids| ids.contains(&entry.ordinal))
        })
        .map(|entry| {
            entry
                .name
                .replace('\\', "/")
                .trim_end_matches('/')
                .to_owned()
        })
        .collect::<HashSet<_>>();
    if let Some(ordinals) = &ordinals {
        let found = metadata
            .entries
            .iter()
            .filter(|entry| ordinals.contains(&entry.ordinal))
            .count();
        if found != ordinals.len() {
            return unknown("选定成员不在当前源快照中");
        }
    }
    let mut payload = 0u64;
    let mut allocation = 0u64;
    let mut directories = HashSet::new();
    let mut incomplete = false;
    let mut directory_bytes = 0usize;
    let mut roots = HashSet::new();
    let mut has_contents = false;
    for entry in &metadata.entries {
        let name = entry.name.replace('\\', "/");
        let descendant = name
            .match_indices('/')
            .any(|(index, _)| prefixes.contains(&name[..index]));
        if ordinals
            .as_ref()
            .is_some_and(|ids| !ids.contains(&entry.ordinal))
            && !descendant
        {
            continue;
        }
        if entry
            .warning_codes
            .iter()
            .any(|warning| matches!(warning.as_str(), "UnsafePath" | "OutputCollision"))
        {
            return unknown("成员路径存在安全警告；不能据此确认空间可用");
        }
        if !matches!(entry.kind.as_str(), "File" | "Directory") {
            continue;
        }
        let components = name.trim_end_matches('/').split('/').collect::<Vec<_>>();
        if components.len() > 64
            || components
                .iter()
                .any(|part| part.is_empty() || matches!(*part, "." | ".."))
        {
            return unknown("成员目录层级或路径超出原安全范围");
        }
        roots.insert(components[0].to_owned());
        has_contents = true;
        let count = components
            .len()
            .saturating_sub(usize::from(entry.kind == "File"));
        for end in 1..=count {
            let directory = components[..end].join("/");
            if !directories.contains(&directory) {
                let Some(next) = directory_bytes.checked_add(directory.len()) else {
                    return unknown("目录估算超出有界内存范围");
                };
                if next > 16 << 20 || directories.len() >= 65_536 {
                    return unknown("目录估算超出有界内存范围");
                }
                directory_bytes = next;
                directories.insert(directory);
            }
        }
        if entry.kind == "Directory" {
            continue;
        }
        let Some(size) = entry.size else {
            incomplete = true;
            continue;
        };
        let Some(next) = payload.checked_add(size) else {
            return unknown("声明大小求和超出可表示范围");
        };
        payload = next;
        let Some(next) = round(size, unit).and_then(|bytes| allocation.checked_add(bytes)) else {
            return unknown("分配空间估算溢出");
        };
        allocation = next;
    }
    // Conservative directory allocation allowance, not a configurable safety reserve or exact peak.
    let one_directory = roots.len() == 1
        && roots
            .iter()
            .next()
            .is_some_and(|root| directories.contains(root));
    let wrapper = usize::from(
        has_contents
            && match extract.strategy {
                ArchiveJobStrategy::Here => false,
                ArchiveJobStrategy::Name => true,
                ArchiveJobStrategy::Smart => {
                    !(one_directory || roots.len() == 1 && !extract.single_file_folder)
                }
            },
    );
    let Some(extra) = (directories.len() as u64)
        .checked_add(wrapper as u64)
        .and_then(|count| count.checked_mul(unit))
    else {
        return unknown("目录空间估算溢出");
    };
    let Some(allocation) = allocation.checked_add(extra) else {
        return unknown("分配空间估算溢出");
    };
    Estimate {
        payload: (!incomplete).then_some(payload),
        allocation: (!incomplete).then_some(allocation),
        lower_bound: payload,
        limitation: incomplete.then_some("部分成员大小未知；仅能确认已知成员声明字节下界"),
    }
}

#[derive(Default)]
pub(super) struct FormState {
    pub key: Option<ArchiveJobRequest>,
    pub epoch: u64,
    pub pending: bool,
    pub plan: Option<Plan>,
    pub error: Option<String>,
    pub stale: bool,
    pub waiting: bool,
    pub options_open: bool,
    pub password_open: bool,
}
impl FormState {
    pub fn invalidate(&mut self, key: Option<ArchiveJobRequest>) -> bool {
        if self.key == key {
            return false;
        }
        self.stale = self.plan.is_some() || self.key.is_some();
        self.key = key;
        self.epoch = self.epoch.wrapping_add(1).max(1);
        self.pending = false;
        self.waiting = false;
        self.plan = None;
        self.error = None;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn observation(count: Option<u32>, missing: bool) -> ArchiveSpaceObservation {
        ArchiveSpaceObservation {
            version: 1,
            operation_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            generation: 1,
            available_bytes: 0,
            allocation_bytes: 4096,
            target_identity: "v1:opaque".into(),
            volume_identity: "v1:volume".into(),
            destination_missing: missing,
            missing_destination_directories: count,
            source_matches: true,
            source_set: None,
        }
    }
    #[test]
    fn hz1_ac02_target_allowance_is_once_per_native_missing_component() {
        let data = data(vec![entry(0, "zero.txt", "File", Some(0))]);
        for (strategy, wrapper) in [
            (ArchiveJobStrategy::Here, 0),
            (ArchiveJobStrategy::Name, 4096),
            (ArchiveJobStrategy::Smart, 0),
        ] {
            let request = request(&data, None, strategy);
            for count in 0..=2 {
                let observed = observation(Some(count), count != 0);
                let estimate = estimate_observed(&request, Some(&data), &observed);
                assert_eq!(estimate.allocation, Some(wrapper + u64::from(count) * 4096));
                let plan = Plan {
                    observation: observed,
                    estimate,
                    concurrent_jobs: 0,
                };
                assert_eq!(plan.shortfall(), Some(wrapper + u64::from(count) * 4096));
            }
        }
        let mut smart = request(&data, None, ArchiveJobStrategy::Smart);
        smart.extract.as_mut().expect("extract").single_file_folder = true;
        assert_eq!(
            estimate_observed(&smart, Some(&data), &observation(Some(2), true)).allocation,
            Some(12288)
        );
    }
    #[test]
    fn hz1_ac02_missing_unknown_inconsistent_and_overflow_remain_unknown() {
        let mut data = data(vec![entry(0, "zero.txt", "File", Some(0))]);
        let request = request(&data, None, ArchiveJobStrategy::Here);
        for (count, missing) in [
            (None, true),
            (Some(0), true),
            (Some(2), false),
            (Some(65), true),
        ] {
            assert!(
                estimate_observed(&request, Some(&data), &observation(count, missing))
                    .allocation
                    .is_none()
            );
        }
        assert_eq!(
            estimate_observed(&request, Some(&data), &observation(None, false)).allocation,
            Some(0),
            "legacy existing target has no missing components"
        );
        data.entries[0].size = None;
        assert!(
            estimate_observed(&request, Some(&data), &observation(Some(2), true))
                .allocation
                .is_none()
        );
        data.entries[0].size = Some(u64::MAX - 4095);
        assert!(
            estimate_observed(&request, Some(&data), &observation(Some(1), true))
                .allocation
                .is_none(),
            "adding target allowance is checked"
        );
        assert!(
            estimate_observed(&request, None, &observation(Some(2), true))
                .allocation
                .is_none()
        );
    }
    fn data(entries: Vec<ArchiveReadEntry>) -> Metadata {
        Metadata {
            browser: ArchiveBrowserContext {
                snapshot_id: Uuid::new_v4(),
                format: "Zip".into(),
                comment: String::new(),
                source: nxtools_protocol::archive_browser::ArchiveSourceBinding {
                    native: "v1:fixture".into(),
                    digest: [0; 32],
                    size: 999_999,
                    modified_unix_ns: 0,
                },
                sevenz_edit: None,
                tar_edit: None,
            },
            entries,
        }
    }
    fn entry(ordinal: u64, name: &str, kind: &str, size: Option<u64>) -> ArchiveReadEntry {
        serde_json::from_value(serde_json::json!({"ordinal":ordinal,"name":name,"kind":kind,"size":size,"packed_size":null,
            "encrypted":false,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001})).expect("metadata")
    }
    fn request(
        data: &Metadata,
        selection: Option<Vec<u64>>,
        strategy: ArchiveJobStrategy,
    ) -> ArchiveJobRequest {
        ArchiveJobRequest {
            kind: ArchiveJobKind::Extract,
            paths: vec![r"C:\owned\archive.zip".into()],
            destination: Some(r"C:\owned\target".into()),
            codepage: 0,
            limits: Default::default(),
            extract: Some(ArchiveJobExtract {
                strategy,
                single_file_folder: false,
                conflict: ArchiveJobConflict::Ask,
                copy_motw: true,
                selection: selection.map(|ordinals| ArchiveJobSelection {
                    browser: data.browser.clone(),
                    ordinals,
                }),
                reclamation: None,
            }),
            create: None,
            modify: None,
            readmission: None,
        }
    }
    #[test]
    fn hz1_empty_named_output_still_counts_its_real_wrapper_directory() {
        let data = data(vec![entry(0, "empty.txt", "File", Some(0))]);
        let result = estimate(
            &request(&data, None, ArchiveJobStrategy::Name),
            Some(&data),
            4096,
        );
        assert_eq!(result.payload, Some(0));
        assert_eq!(
            result.allocation,
            Some(4096),
            "named extraction of a zero-length file still needs the wrapper allocation allowance"
        );
    }
    #[test]
    fn hz1_known_all_selected_directory_and_no_source_credit() {
        let data = data(vec![
            entry(0, "a", "Directory", None),
            entry(1, "a/one", "File", Some(1)),
            entry(2, "b", "File", Some(4097)),
        ]);
        let all = estimate(
            &request(&data, None, ArchiveJobStrategy::Here),
            Some(&data),
            4096,
        );
        assert_eq!(all.payload, Some(4098));
        assert_eq!(all.allocation, Some(16384));
        let selected = estimate(
            &request(&data, Some(vec![0]), ArchiveJobStrategy::Here),
            Some(&data),
            4096,
        );
        assert_eq!(selected.payload, Some(1));
        assert_eq!(selected.allocation, Some(8192));
        assert_eq!(
            estimate(
                &request(&data, Some(vec![2]), ArchiveJobStrategy::Here),
                Some(&data),
                4096
            )
            .allocation,
            Some(8192)
        );
        assert_eq!(
            data.browser.source.size, 999_999,
            "source allocation not charged or credited"
        );
    }
    #[test]
    fn hz1_unknown_empty_large_and_overflow_never_invent_fit() {
        let mut data = data(vec![
            entry(0, "unknown", "File", None),
            entry(1, "known", "File", Some(4096)),
        ]);
        let result = estimate(
            &request(&data, None, ArchiveJobStrategy::Here),
            Some(&data),
            4096,
        );
        assert!(result.allocation.is_none() && result.payload.is_none());
        assert_eq!(result.lower_bound, 4096);
        data.entries.clear();
        assert_eq!(
            estimate(
                &request(&data, None, ArchiveJobStrategy::Name),
                Some(&data),
                4096
            )
            .allocation,
            Some(0)
        );
        data.entries = vec![entry(0, "big", "File", Some(1 << 40))];
        assert_eq!(
            estimate(
                &request(&data, None, ArchiveJobStrategy::Here),
                Some(&data),
                4096
            )
            .allocation,
            Some(1 << 40)
        );
        data.entries = vec![entry(0, "overflow", "File", Some(u64::MAX))];
        assert!(
            estimate(
                &request(&data, None, ArchiveJobStrategy::Here),
                Some(&data),
                4096
            )
            .allocation
            .is_none()
        );
        assert!(
            estimate(
                &request(&data, None, ArchiveJobStrategy::Here),
                Some(&data),
                0
            )
            .allocation
            .is_none()
        );
    }
    #[test]
    fn hz1_zero_and_exact_capacity_keep_fixed_zero_reserve_and_budget() {
        let data = data(vec![entry(0, "payload", "File", Some(1))]);
        let request = request(&data, None, ArchiveJobStrategy::Here);
        let estimate = estimate(&request, Some(&data), 4096);
        let mut plan = Plan {
            estimate,
            observation: ArchiveSpaceObservation {
                version: 1,
                operation_id: Uuid::new_v4(),
                session_id: Uuid::new_v4(),
                generation: 1,
                available_bytes: 0,
                allocation_bytes: 4096,
                target_identity: "v1:target".into(),
                volume_identity: "v1:volume".into(),
                destination_missing: false,
                missing_destination_directories: Some(0),
                source_matches: true,
                source_set: None,
            },
            concurrent_jobs: 2,
        };
        assert_eq!(plan.shortfall(), Some(4096));
        assert_eq!(plan.remaining(), Some(0));
        plan.observation.available_bytes = 4096;
        assert_eq!(plan.shortfall(), Some(0));
        assert_eq!(plan.remaining(), Some(0));
        plan.observation.available_bytes = 8192;
        assert_eq!(plan.remaining(), Some(4096));
        assert_eq!(
            request.limits,
            ArchiveJobLimits::default(),
            "advisory calculation never widens output budget"
        );
    }
}
