use serde::{Deserialize, Serialize};

/// Complete packaged-application snapshot from the current Windows user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledApplications {
    pub applications: Vec<InstalledApplication>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledApplication {
    pub name: String,
    pub target: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PlatformCommand;

    #[test]
    fn installed_applications_wire_matches_native_snapshot() {
        assert_eq!(
            serde_json::to_value(PlatformCommand::QueryInstalledApplications)
                .expect("command JSON"),
            serde_json::json!({"command": "query_installed_applications"})
        );
        let json = r#"{"applications":[{"name":"计算器","target":"shell:AppsFolder\\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"}]}"#;
        let snapshot: InstalledApplications = serde_json::from_str(json).expect("native snapshot");
        assert_eq!(snapshot.applications[0].name, "计算器");
        assert_eq!(
            serde_json::to_value(snapshot).expect("snapshot JSON"),
            serde_json::from_str::<serde_json::Value>(json).expect("expected JSON")
        );
        assert!(serde_json::from_str::<InstalledApplications>("{}").is_err());
        assert!(
            serde_json::from_str::<InstalledApplications>(
                r#"{"applications":[{"name":"missing target"}]}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<InstalledApplications>(r#"{"applications":[]}"#)
                .expect("empty snapshot")
                .applications
                .is_empty()
        );
    }
}
