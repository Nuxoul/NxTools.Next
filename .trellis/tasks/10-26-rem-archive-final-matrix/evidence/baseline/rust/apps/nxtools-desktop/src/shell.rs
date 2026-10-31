#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum MainModule {
    #[default]
    Downloads,
    QuickFind,
    Clipboard,
    Extensions,
    Archive,
    Tools,
}

impl MainModule {
    pub(crate) const ALL: [Self; 6] = [
        Self::Downloads,
        Self::QuickFind,
        Self::Clipboard,
        Self::Extensions,
        Self::Archive,
        Self::Tools,
    ];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Downloads => "下载",
            Self::QuickFind => "速启",
            Self::Clipboard => "剪贴板",
            Self::Extensions => "扩展",
            Self::Archive => "压缩",
            Self::Tools => "其他工具",
        }
    }

    pub(crate) const fn description(self) -> &'static str {
        match self {
            Self::Downloads => "多线程下载与任务调度",
            Self::QuickFind => "文件、应用与命令快速检索",
            Self::Clipboard => "历史、分类与粘贴行为管理",
            Self::Extensions => "管理下载、速启、剪贴板及工具扩展",
            Self::Archive => "压缩、解压与压缩包浏览",
            Self::Tools => "链接识别等扩展小工具",
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct ShellState {
    active_module: MainModule,
}

impl ShellState {
    pub(crate) const fn active_module(&self) -> MainModule {
        self.active_module
    }

    pub(crate) fn activate(&mut self, module: MainModule) {
        self.active_module = module;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_navigation_has_the_approved_six_stable_entries() {
        assert_eq!(
            MainModule::ALL.map(MainModule::label).as_slice(),
            ["下载", "速启", "剪贴板", "扩展", "压缩", "其他工具"]
        );
    }

    #[test]
    fn main_navigation_places_extensions_immediately_after_clipboard() {
        assert_eq!(
            MainModule::ALL,
            [
                MainModule::Downloads,
                MainModule::QuickFind,
                MainModule::Clipboard,
                MainModule::Extensions,
                MainModule::Archive,
                MainModule::Tools,
            ]
        );
        assert_eq!(MainModule::Downloads.label(), "下载");
        assert_eq!(MainModule::Tools.label(), "其他工具");
        assert_eq!(MainModule::QuickFind.label(), "速启");
        assert_eq!(MainModule::Clipboard.label(), "剪贴板");
        assert_eq!(MainModule::Extensions.label(), "扩展");
    }

    #[test]
    fn shell_switches_module_without_recreating_module_state() {
        let mut shell = ShellState::default();
        assert_eq!(shell.active_module(), MainModule::Downloads);

        shell.activate(MainModule::Clipboard);
        assert_eq!(shell.active_module(), MainModule::Clipboard);

        shell.activate(MainModule::QuickFind);
        assert_eq!(shell.active_module(), MainModule::QuickFind);
    }
}
