use super::*;

const SHELL_APPLICATION_PREFIX: &str = "shell:AppsFolder\\";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledApplication {
    pub name: String,
    pub target: String,
}

pub fn is_packaged_application_target(target: &str) -> bool {
    target
        .strip_prefix(SHELL_APPLICATION_PREFIX)
        .is_some_and(|id| {
            id.len() <= 128
                && id.split_once('!').is_some_and(|(family, application)| {
                    !family.is_empty() && !application.is_empty() && !application.contains('!')
                })
                && id.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'!')
                })
        })
}

struct WindowsApplicationAliases {
    family: &'static str,
    fallback: Option<&'static str>,
    aliases: &'static [&'static str],
}

const WINDOWS_APPLICATIONS: &[WindowsApplicationAliases] = &[
    WindowsApplicationAliases {
        family: "Microsoft.WindowsCalculator",
        fallback: Some("calc.exe"),
        aliases: &["计算器", "calculator", "windows calculator", "calc"],
    },
    WindowsApplicationAliases {
        family: "Microsoft.Paint",
        fallback: Some("mspaint.exe"),
        aliases: &["画图", "paint", "mspaint"],
    },
    WindowsApplicationAliases {
        family: "Microsoft.WindowsNotepad",
        fallback: Some("notepad.exe"),
        aliases: &["记事本", "notepad", "windows notepad"],
    },
    WindowsApplicationAliases {
        family: "Microsoft.ScreenSketch",
        fallback: None,
        aliases: &[
            "截图工具",
            "截图和草图",
            "snipping tool",
            "snip",
            "snippingtool",
        ],
    },
    WindowsApplicationAliases {
        family: "Microsoft.Windows.Photos",
        fallback: None,
        aliases: &["照片", "photos", "microsoft photos"],
    },
    WindowsApplicationAliases {
        family: "Microsoft.WindowsCamera",
        fallback: None,
        aliases: &["相机", "camera", "windows camera"],
    },
    WindowsApplicationAliases {
        family: "Microsoft.WindowsSoundRecorder",
        fallback: None,
        aliases: &["录音机", "录音笔", "sound recorder", "voice recorder"],
    },
    WindowsApplicationAliases {
        family: "Microsoft.WindowsAlarms",
        fallback: None,
        aliases: &["时钟", "闹钟和时钟", "clock", "alarms"],
    },
    WindowsApplicationAliases {
        family: "Microsoft.WindowsTerminal",
        fallback: None,
        aliases: &["终端", "terminal", "windows terminal", "wt"],
    },
    WindowsApplicationAliases {
        family: "Microsoft.MicrosoftStickyNotes",
        fallback: None,
        aliases: &["便笺", "sticky notes"],
    },
    WindowsApplicationAliases {
        family: "Microsoft.WindowsStore",
        fallback: None,
        aliases: &["应用商店", "微软商店", "microsoft store", "store"],
    },
    WindowsApplicationAliases {
        family: "Microsoft.ZuneMusic",
        fallback: None,
        aliases: &["媒体播放器", "media player", "groove music"],
    },
    WindowsApplicationAliases {
        family: "Microsoft.WindowsFeedbackHub",
        fallback: None,
        aliases: &["反馈中心", "feedback hub"],
    },
    WindowsApplicationAliases {
        family: "Microsoft.GetHelp",
        fallback: None,
        aliases: &["获取帮助", "get help"],
    },
    WindowsApplicationAliases {
        family: "Microsoft.Microsoft3DViewer",
        fallback: None,
        aliases: &["3d 查看器", "3d viewer"],
    },
];

fn descriptor(target: &str) -> Option<&'static WindowsApplicationAliases> {
    if let Some(id) = target.strip_prefix(SHELL_APPLICATION_PREFIX) {
        let family = id.split_once('_')?.0;
        WINDOWS_APPLICATIONS
            .iter()
            .find(|entry| entry.family.eq_ignore_ascii_case(family))
    } else {
        WINDOWS_APPLICATIONS.iter().find(|entry| {
            entry
                .fallback
                .is_some_and(|fallback| fallback.eq_ignore_ascii_case(target))
        })
    }
}

pub(super) fn system_application_aliases(entry: &ApplicationEntry) -> &'static [&'static str] {
    if entry.source != ApplicationSource::System {
        return &[];
    }
    if entry.target == "ms-settings:" {
        return &["settings", "windows settings", "系统设置"];
    }
    descriptor(&entry.target).map_or(&[], |entry| entry.aliases)
}

impl FinderCatalog {
    pub fn replace_installed_applications(
        &mut self,
        applications: Vec<InstalledApplication>,
    ) -> Result<()> {
        let mut entries = Vec::with_capacity(applications.len());
        let mut seen = HashSet::new();
        for application in applications {
            anyhow::ensure!(
                !application.name.trim().is_empty()
                    && is_packaged_application_target(&application.target),
                "invalid installed application snapshot"
            );
            if seen.insert(normalize_target(&application.target)) {
                entries.push(ApplicationEntry {
                    id: stable_entry_id(&application.target),
                    name: application.name,
                    target: application.target,
                    source: ApplicationSource::System,
                });
            }
        }
        self.installed_entries = entries;
        self.rebuild_application_entries();
        Ok(())
    }

    pub(super) fn rebuild_application_entries(&mut self) {
        let shortcut_names = self
            .root_entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.source,
                    ApplicationSource::StartMenu | ApplicationSource::Desktop
                )
            })
            .map(|entry| entry.name.to_lowercase())
            .collect::<HashSet<_>>();
        let shortcut_fallbacks = WINDOWS_APPLICATIONS
            .iter()
            .filter(|entry| {
                entry
                    .aliases
                    .iter()
                    .any(|alias| shortcut_names.contains(*alias))
            })
            .filter_map(|entry| entry.fallback);
        let superseded = shortcut_fallbacks
            .chain(
                self.installed_entries
                    .iter()
                    .filter_map(|entry| descriptor(&entry.target).and_then(|entry| entry.fallback)),
            )
            .collect::<HashSet<_>>();
        self.entries = self
            .root_entries
            .iter()
            .filter(|entry| {
                entry.source != ApplicationSource::System
                    || !superseded.contains(entry.target.as_str())
            })
            .chain(self.installed_entries.iter())
            .cloned()
            .collect();
        self.match_materials = self
            .entries
            .iter()
            .map(ApplicationMatchMaterial::build)
            .collect();
        self.status.indexed_items = self.entries.len();
    }
}
