use super::*;
use std::path::Path;

use gpui_kit::component::{
    WindowExt as _,
    dialog::{DialogAction, DialogClose, DialogFooter},
    diff::{Diff, DiffFile, DiffMode, DiffState},
    list::ListItem,
};
use rusqlite::{Connection, OptionalExtension as _, TransactionBehavior, params};

pub(super) struct Store(Connection);

use gpui_kit::StatefulInteractiveElement as _;
pub(super) struct Version {
    pub id: i64,
    pub name: Option<String>,
    date: String,
    event: String,
}

impl Store {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(2))?;
        connection.execute_batch("CREATE TABLE IF NOT EXISTS markdown_version(
            id INTEGER PRIMARY KEY AUTOINCREMENT, document TEXT NOT NULL, content BLOB NOT NULL,
            date TEXT NOT NULL, event TEXT NOT NULL, name TEXT
        ); CREATE INDEX IF NOT EXISTS markdown_version_document ON markdown_version(document, id DESC);")?;
        Ok(Self(connection))
    }

    fn system() -> anyhow::Result<Self> {
        Self::open(
            &nxtools_core::AppPaths::discover()?
                .data_dir
                .join("markdown-versions.sqlite3"),
        )
    }

    pub fn record(&mut self, path: &Path, bytes: &[u8], event: &str) -> anyhow::Result<i64> {
        let key = path.to_string_lossy();
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let last = tx.query_row("SELECT id, content FROM markdown_version WHERE document=?1 ORDER BY id DESC LIMIT 1", [&key], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))).optional()?;
        if let Some((id, previous)) = last
            && previous == bytes
        {
            return Ok(id);
        }
        tx.execute(
            "INSERT INTO markdown_version(document, content, date, event) VALUES (?1,?2,?3,?4)",
            params![key, bytes, chrono::Utc::now().to_rfc3339(), event],
        )?;
        let id = tx.last_insert_rowid();
        tx.execute("DELETE FROM markdown_version WHERE id IN (SELECT id FROM markdown_version WHERE document=?1 AND name IS NULL ORDER BY id DESC LIMIT -1 OFFSET 32)", [&key])?;
        tx.commit()?;
        Ok(id)
    }

    pub fn list(&self, path: &Path) -> anyhow::Result<Vec<Version>> {
        let mut statement = self.0.prepare(
            "SELECT id,name,date,event FROM markdown_version WHERE document=?1 ORDER BY id DESC",
        )?;
        let versions = statement
            .query_map([path.to_string_lossy()], |row| {
                Ok(Version {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    date: row.get(2)?,
                    event: row.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(versions)
    }

    pub fn bytes(&self, path: &Path, id: i64) -> anyhow::Result<Vec<u8>> {
        Ok(self.0.query_row(
            "SELECT content FROM markdown_version WHERE document=?1 AND id=?2",
            params![path.to_string_lossy(), id],
            |row| row.get(0),
        )?)
    }

    pub fn rename(&self, path: &Path, id: i64, name: &str) -> anyhow::Result<()> {
        let name = name.trim();
        anyhow::ensure!(
            !name.is_empty() && name.chars().count() <= 80,
            "invalid version name"
        );
        anyhow::ensure!(
            self.0.execute(
                "UPDATE markdown_version SET name=?1 WHERE id=?2 AND document=?3",
                params![name, id, path.to_string_lossy()]
            )? == 1,
            "version not found"
        );
        Ok(())
    }

    pub fn restore(
        &mut self,
        path: &Path,
        id: i64,
        current: &[u8],
    ) -> anyhow::Result<(Document, bool)> {
        let replacement = self.bytes(path, id)?;
        let actual =
            document::read(path.to_owned()).map_err(|error| anyhow::anyhow!(error.code()))?;
        anyhow::ensure!(actual.original() == current, "document changed externally");
        self.record(path, current, "before_restore")?;
        let restored = document::replace(path.to_owned(), current, replacement)
            .map_err(|error| anyhow::anyhow!(error.code()))?;
        let recorded = self.record(path, &restored.original(), "restored").is_ok();
        Ok((restored, recorded))
    }
}

pub(super) fn observe(document: &Document, event: &'static str, correlation: Uuid) -> bool {
    if cfg!(test) {
        return true;
    }
    let started = Instant::now();
    let result = Store::system()
        .and_then(|mut store| store.record(&document.path, &document.original(), event));
    tracing::info!(event = "markdown.history.recorded", correlation_id = %correlation, reason = event, success = result.is_ok(), elapsed_ms = started.elapsed().as_millis());
    result.is_ok()
}

pub(super) struct HistoryView {
    tab: usize,
    versions: Vec<Version>,
    selected: Option<i64>,
    compare_to: Option<i64>,
    name: Entity<InputState>,
    diff: Option<Entity<DiffState>>,
}

impl Reader {
    pub(super) fn history_button(&self, cx: &mut Context<Self>) -> Button {
        Button::new("markdown-history")
            .label(if self.history.is_some() {
                "返回文档"
            } else {
                "历史版本"
            })
            .small()
            .outline()
            .disabled(self.active_tab().is_none() || self.history_work.is_some())
            .on_click(cx.listener(|this, _, window, cx| {
                if this.history.take().is_some() {
                    cx.notify();
                } else {
                    this.open_history(window, cx);
                }
            }))
    }

    fn open_history(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else { return };
        let id = tab.id;
        let path = tab.document.path.clone();
        let worker = cx.background_spawn(async move { Store::system()?.list(&path) });
        self.history_work = Some(cx.spawn_in(window, async move |owner, cx| {
            let result = worker.await;
            let _ = owner.update_in(cx, |this, window, cx| {
                this.history_work = None;
                if !this.active_tab().is_some_and(|tab| tab.id == id) {
                    return;
                }
                match result {
                    Ok(versions) => {
                        let name = cx.new(|cx| InputState::new(window, cx).placeholder("版本名称"));
                        this.history = Some(HistoryView {
                            tab: id,
                            versions,
                            selected: None,
                            compare_to: None,
                            name,
                            diff: None,
                        });
                        this.problem = None;
                    }
                    Err(_) => this.problem = Some("无法读取本地版本历史。"),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn compare_version(&mut self, selected: i64, window: &mut Window, cx: &mut Context<Self>) {
        if self.history_work.is_some() {
            return;
        }
        let (Some(tab), Some(history)) = (self.active_tab(), &self.history) else {
            return;
        };
        let tab_id = tab.id;
        let path = tab.document.path.clone();
        let current = tab.document.original();
        let compare_to = history.compare_to;
        let operation = Uuid::new_v4();
        let started = Instant::now();
        let worker = cx.background_spawn(async move {
            let store = Store::system()?;
            let before = String::from_utf8(store.bytes(&path, selected)?)?;
            let after = String::from_utf8(match compare_to {
                Some(id) => store.bytes(&path, id)?,
                None => current,
            })?;
            let patch = similar::TextDiff::from_lines(&before, &after)
                .unified_diff()
                .header("a/document.md", "b/document.md")
                .to_string();
            if patch.is_empty() {
                return Ok::<_, anyhow::Error>(Vec::new());
            }
            Ok(DiffFile::parse(&patch)?
                .into_iter()
                .map(|file| file.with_language("markdown"))
                .collect())
        });
        self.history_work = Some(cx.spawn_in(window, async move |owner, cx| {
            let result = worker.await;
            let _ = owner.update_in(cx, |this, _, cx| {
                this.history_work = None;
                let Some(history) = this.history.as_mut().filter(|history| history.tab == tab_id) else { return };
                history.selected = Some(selected);
                match result {
                    Ok(files) if files.is_empty() => history.diff = None,
                    Ok(files) => history.diff = Some(cx.new(|cx| DiffState::new(files, cx).with_mode(DiffMode::Split))),
                    Err(_) => { history.diff = None; this.problem = Some("无法比较所选版本。"); }
                }
                tracing::info!(event = "markdown.history.compare.finished", correlation_id = %operation, elapsed_ms = started.elapsed().as_millis());
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn rename_version(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.history_work.is_some() {
            return;
        }
        let (Some(tab), Some(history)) = (self.active_tab(), &self.history) else {
            return;
        };
        let Some(id) = history.selected else { return };
        let name = history.name.read(cx).value().to_string();
        if name.trim().is_empty() {
            return;
        }
        let tab_id = tab.id;
        let path = tab.document.path.clone();
        let worker = cx.background_spawn(async move {
            let store = Store::system()?;
            store.rename(&path, id, &name)?;
            store.list(&path)
        });
        self.history_work = Some(cx.spawn_in(window, async move |owner, cx| {
            let result = worker.await;
            let _ = owner.update_in(cx, |this, _, cx| {
                this.history_work = None;
                let Some(history) = this
                    .history
                    .as_mut()
                    .filter(|history| history.tab == tab_id)
                else {
                    return;
                };
                match result {
                    Ok(versions) => history.versions = versions,
                    Err(_) => this.problem = Some("版本名称保存失败（最多 80 个字符）。"),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn confirm_restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab().filter(|tab| !tab.draft.dirty()) else {
            return;
        };
        let Some(version) = self.history.as_ref().and_then(|history| history.selected) else {
            return;
        };
        let tab = tab.id;
        let owner = cx.weak_entity();
        window.open_dialog(cx, move |dialog, _, _| {
            let owner = owner.clone();
            dialog
                .title("恢复所选版本？")
                .child("当前已保存文件会先记入历史版本。")
                .footer(
                    DialogFooter::new()
                        .child(DialogClose::new().trigger(|button| button.label("取消").outline()))
                        .child(
                            DialogAction::new().child(
                                Button::new("markdown-history-confirm")
                                    .label("恢复版本")
                                    .primary()
                                    .on_click(|event, window, cx| {
                                        if event.is_keyboard() {
                                            window.dispatch_action(
                                                Box::new(gpui_kit::component::dialog::Confirm {
                                                    secondary: false,
                                                }),
                                                cx,
                                            );
                                        }
                                    }),
                            ),
                        ),
                )
                .on_ok(move |_, window, cx| {
                    let owner = owner.clone();
                    window.defer(cx, move |window, cx| {
                        let _ = owner.update(cx, |this, cx| {
                            this.restore_version(tab, version, window, cx)
                        });
                    });
                    true
                })
        });
    }

    fn restore_version(
        &mut self,
        tab_id: usize,
        version: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.history_work.is_some() || self.saving.is_some() {
            return;
        }
        let Some(tab) = self
            .active_tab()
            .filter(|tab| tab.id == tab_id && !tab.draft.dirty())
        else {
            return;
        };
        let path = tab.document.path.clone();
        let bytes = tab.document.original();
        let correlation = Uuid::new_v4();
        let started = Instant::now();
        tracing::info!(event = "markdown.history.restore.started", correlation_id = %correlation);
        let worker =
            cx.background_spawn(async move { Store::system()?.restore(&path, version, &bytes) });
        self.saving = Some(cx.spawn_in(window, async move |owner, cx| {
            let result = worker.await;
            let _ = owner.update_in(cx, |this, window, cx| {
                this.saving = None;
                let Some(tab) = this.tabs.iter_mut().find(|tab| tab.id == tab_id) else { return };
                tracing::info!(event = "markdown.history.restore.finished", correlation_id = %correlation, success = result.is_ok(), elapsed_ms = started.elapsed().as_millis());
                match result {
                    Ok((restored, recorded)) => {
                        if restored.text != tab.document.text {
                            tab.navigation.content_changed();
                        }
                        tab.draft.refresh_clean(&restored.text, window, cx);
                        tab.view.update(cx, |view, cx| view.set_text(&restored.text, cx));
                        tab.outline.update(cx, |tree, cx| tree.set_items(outline_items(&restored.headings), cx));
                        tab.watch_stamp = restored.stamp;
                        tab.document = restored;
                        tab.problem = None;
                        this.history = None;
                        this.notice = Some("已恢复所选版本。");
                        this.problem = (!recorded).then_some("文件已恢复，但新版本未能写入历史。");
                    }
                    Err(_) => this.problem = Some("无法恢复版本。请检查文件是否已被外部修改或无法写入。"),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    pub(super) fn render_history(
        &self,
        history: &HistoryView,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let enabled = self.history_work.is_none() && self.saving.is_none();
        let can_restore = enabled
            && history.selected.is_some()
            && self.active_tab().is_some_and(|tab| !tab.draft.dirty());
        let theme = Theme::active(cx);
        let timeline = div()
            .id("markdown-history-list")
            .w(px(245.))
            .flex_none()
            .p_2()
            .min_h_0()
            .overflow_y_scroll()
            .border_r_1()
            .border_color(theme.border_subtle)
            .children(history.versions.iter().map(|version| {
                let id = version.id;
                let reason = match version.event.as_str() {
                    "opened" => "打开",
                    "saved" => "保存",
                    "external" => "外部修改",
                    "before_restore" => "恢复前",
                    "restored" => "恢复",
                    _ => "版本",
                };
                let date = chrono::DateTime::parse_from_rfc3339(&version.date)
                    .ok()
                    .map(|time| {
                        time.with_timezone(&chrono::Local)
                            .format("%Y-%m-%d %H:%M:%S")
                            .to_string()
                    })
                    .unwrap_or_default();
                ListItem::new(("markdown-version", id as usize))
                    .selected(history.selected == Some(id))
                    .disabled(!enabled)
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(version.name.clone().unwrap_or_else(|| reason.to_owned()))
                            .child(div().text_xs().text_color(theme.text_muted).child(date)),
                    )
                    .on_click(cx.listener(move |_, _, window, cx| {
                        cx.defer_in(window, move |this, window, cx| {
                            this.compare_version(id, window, cx)
                        })
                    }))
            }));
        div()
            .flex()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .child(timeline)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .p_2()
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .child(Input::new(&history.name).aria_label("版本名称")),
                            )
                            .child(
                                Button::new("markdown-name-version")
                                    .label("命名")
                                    .small()
                                    .outline()
                                    .disabled(!enabled || history.selected.is_none())
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.rename_version(window, cx)
                                    })),
                            )
                            .child(
                                Button::new("markdown-version-baseline")
                                    .label(if history.compare_to.is_some() {
                                        "与当前比较"
                                    } else {
                                        "设为对比目标"
                                    })
                                    .small()
                                    .outline()
                                    .disabled(!enabled || history.selected.is_none())
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        if let Some(history) = &mut this.history
                                            && let Some(id) = history.selected
                                        {
                                            history.compare_to = if history.compare_to.is_some() {
                                                None
                                            } else {
                                                Some(id)
                                            };
                                            this.compare_version(id, window, cx);
                                        }
                                    })),
                            )
                            .child(
                                Button::new("markdown-restore")
                                    .label("恢复…")
                                    .small()
                                    .outline()
                                    .disabled(!can_restore)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.confirm_restore(window, cx)
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .min_w_0()
                            .when_some(history.diff.as_ref(), |body, diff| {
                                body.child(Diff::new(diff).size_full())
                            })
                            .when(history.diff.is_none(), |body| {
                                body.p_4().child(if history.versions.is_empty() {
                                    "还没有已保存版本。"
                                } else if history.selected.is_none() {
                                    "选择一个版本查看差异。"
                                } else {
                                    "没有差异。"
                                })
                            }),
                    ),
            )
    }
}
