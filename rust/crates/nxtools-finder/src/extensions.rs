use crate::{
    ApplicationEntry, ApplicationSource, ExternalSearchEntry, FinderCatalog, SearchResult,
    SearchResultAction, command_display_line, normalize, prepare_custom_command,
    validate_custom_command_template,
};
use nxtools_core::CustomCommandSettings;
use nxtools_extensions::{DeclarativeAction, Handler, InstalledPackage, Manifest, PackageRegistry};

/// Validate program templates through the same parser used for manual commands.
/// This performs no process or filesystem operation.
pub fn validate_package_commands(manifest: &Manifest) -> Result<(), String> {
    fn validate(action: &DeclarativeAction) -> Result<(), String> {
        match action {
            DeclarativeAction::Program {
                argument_template, ..
            } => validate_custom_command_template(argument_template),
            DeclarativeAction::Items { items } => {
                for item in items {
                    validate(&item.action)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
    for handler in &manifest.handlers {
        validate(&handler.action)?;
    }
    Ok(())
}

/// Prepare one already validated declarative action, without executing it.
pub fn prepare_package_action(
    action: &DeclarativeAction,
    input: &str,
) -> anyhow::Result<(String, SearchResultAction, String)> {
    match action {
        DeclarativeAction::Website { url_template } => {
            let encoded: String = input
                .as_bytes()
                .iter()
                .map(|byte| {
                    if byte.is_ascii_alphanumeric() || b"-._~".contains(byte) {
                        (*byte as char).to_string()
                    } else {
                        format!("%{byte:02X}")
                    }
                })
                .collect();
            let target = url_template.replace("{query}", &encoded);
            let parsed = url::Url::parse(&target)?;
            anyhow::ensure!(
                matches!(parsed.scheme(), "http" | "https")
                    && parsed.host_str().is_some()
                    && parsed.username().is_empty()
                    && parsed.password().is_none(),
                "invalid extension URL"
            );
            Ok((target.clone(), SearchResultAction::OpenTarget, target))
        }
        DeclarativeAction::Folder { path } => {
            let command = prepare_custom_command(
                &CustomCommandSettings {
                    keyword: "extension".into(),
                    program: path.clone(),
                    ..Default::default()
                },
                None,
            )
            .map_err(anyhow::Error::msg)?;
            Ok((
                command.executable.clone(),
                SearchResultAction::OpenTarget,
                command.executable,
            ))
        }
        DeclarativeAction::Program {
            executable,
            argument_template,
            working_directory,
        } => {
            let command = prepare_custom_command(
                &CustomCommandSettings {
                    keyword: "extension".into(),
                    program: executable.clone(),
                    argument_template: argument_template.clone(),
                    working_directory: working_directory.clone(),
                    ..Default::default()
                },
                Some(input),
            )
            .map_err(anyhow::Error::msg)?;
            let detail = command_display_line(&command);
            Ok((
                command.executable.clone(),
                SearchResultAction::RunCommand { command },
                detail,
            ))
        }
        DeclarativeAction::Script { .. } => {
            anyhow::bail!("script actions require the trusted extension runtime")
        }
        DeclarativeAction::Items { .. } => {
            anyhow::bail!("select an extension list item before execution")
        }
    }
}

impl FinderCatalog {
    /// Whether the ordinary local providers can contribute a result at this limit.
    pub fn has_local_normal_hit(
        &self,
        query: &str,
        manual: &[CustomCommandSettings],
        packages: &PackageRegistry,
        limit: usize,
    ) -> anyhow::Result<bool> {
        if limit == 0 {
            return Ok(false);
        }
        let mut found = false;
        self.visit_search_candidates(query, &[], manual, limit, |_| {
            found = true;
            false
        })?;
        if !found && packages.packages.iter().any(|package| package.enabled) {
            self.visit_package_candidates(query, &normalize(query), packages, |_| {
                found = true;
                false
            });
        }
        Ok(found)
    }

    pub fn search_with_packages_normal(
        &self,
        query: &str,
        external: &[ExternalSearchEntry],
        manual: &[CustomCommandSettings],
        packages: &PackageRegistry,
        limit: usize,
    ) -> anyhow::Result<Vec<SearchResult>> {
        if packages.packages.iter().all(|package| !package.enabled) || limit == 0 {
            return self.search_with_sources_normal(query, external, manual, limit);
        }
        let mut results = self.search_candidates(query, external, manual, limit)?;
        let normalized = normalize(query);
        self.visit_package_candidates(query, &normalized, packages, |result| {
            results.push(result);
            true
        });
        Ok(self.rank_results(results, &normalized, limit))
    }

    fn visit_package_candidates(
        &self,
        query: &str,
        normalized: &str,
        packages: &PackageRegistry,
        mut visit: impl FnMut(SearchResult) -> bool,
    ) {
        for package in packages.packages.iter().filter(|package| package.enabled) {
            for handler in package
                .manifest
                .handlers
                .iter()
                .filter(|handler| handler.feature == "quick_launch")
            {
                let Some(input) = handler_input(package.keywords(handler), query) else {
                    continue;
                };
                match &handler.action {
                    DeclarativeAction::Items { items } => {
                        for item in items.iter().filter(|item| {
                            input.split_whitespace().all(|word| {
                                item.title.to_lowercase().contains(&word.to_lowercase())
                            })
                        }) {
                            if let Ok(result) = self.package_result(
                                package,
                                handler,
                                (&item.id, &item.title, &item.action),
                                "",
                                normalized,
                            ) && !visit(result)
                            {
                                return;
                            }
                        }
                    }
                    action => {
                        if let Ok(result) = self.package_result(
                            package,
                            handler,
                            ("action", &handler.title, action),
                            input,
                            normalized,
                        ) && !visit(result)
                        {
                            return;
                        }
                    }
                }
            }
        }
    }

    pub fn search_with_packages(
        &self,
        query: &str,
        external: &[ExternalSearchEntry],
        manual: &[CustomCommandSettings],
        packages: &PackageRegistry,
        limit: usize,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let normal = self.search_with_packages_normal(query, external, manual, packages, limit)?;
        if !normal.is_empty() {
            Ok(normal)
        } else {
            self.search_with_sources(query, external, manual, limit)
        }
    }

    fn package_result(
        &self,
        package: &InstalledPackage,
        handler: &Handler,
        item: (&str, &str, &DeclarativeAction),
        input: &str,
        normalized_query: &str,
    ) -> anyhow::Result<SearchResult> {
        let (item_id, title, action) = item;
        let (target, action, detail) = prepare_package_action(action, input)?;
        let id = format!(
            "extension:{}/{}/{}",
            package.manifest.id, handler.id, item_id
        );
        let usage = self.usage_for(&id, normalized_query);
        Ok(SearchResult {
            entry: ApplicationEntry {
                id,
                name: title.into(),
                target,
                source: ApplicationSource::CustomCommand,
            },
            action,
            subtitle: Some(format!("{} · {detail}", package.manifest.name)),
            score: 10_000,
            activation_count: usage.activation_count,
            last_activated_at_unix_ms: usage.last_activated_at_unix_ms,
            query_activation_count: usage.query_activation_count,
            last_query_activation_at_unix_ms: usage.last_query_activation_at_unix_ms,
        })
    }
}

fn handler_input<'a>(keywords: &[String], query: &'a str) -> Option<&'a str> {
    let query = query.trim_start();
    if query.is_empty() {
        return None;
    }
    keywords.iter().find_map(|keyword| {
        if keyword
            .get(..query.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(query))
        {
            Some("")
        } else if query
            .get(..keyword.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(keyword))
            && query.as_bytes().get(keyword.len()) == Some(&b' ')
        {
            Some(&query[keyword.len() + 1..])
        } else {
            None
        }
    })
}
