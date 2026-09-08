#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum MainModule {
    #[default]
    Downloads,
    QuickFind,
    Clipboard,
    Tools,
}

impl MainModule {
    pub(crate) const ALL: [Self; 4] = [
        Self::Downloads,
        Self::QuickFind,
        Self::Clipboard,
        Self::Tools,
    ];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Downloads => "下载",
            Self::QuickFind => "速启",
            Self::Clipboard => "剪贴板",
            Self::Tools => "其他工具",
        }
    }

    pub(crate) const fn description(self) -> &'static str {
        match self {
            Self::Downloads => "多线程下载与任务调度",
            Self::QuickFind => "文件、应用与命令快速检索",
            Self::Clipboard => "历史、分类与粘贴行为管理",
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
    fn main_navigation_contains_only_the_three_product_modules() {
        assert_eq!(
            MainModule::ALL,
            [
                MainModule::Downloads,
                MainModule::QuickFind,
                MainModule::Clipboard,
                MainModule::Tools,
            ]
        );
        assert_eq!(MainModule::Downloads.label(), "下载");
        assert_eq!(MainModule::Tools.label(), "其他工具");
        assert_eq!(MainModule::QuickFind.label(), "速启");
        assert_eq!(MainModule::Clipboard.label(), "剪贴板");
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
