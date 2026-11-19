# Grouped download completion

## Goal
Downloads added together produce one useful completion summary instead of a sequence of passive completion windows.

## Requirements
Keep standalone-task completion behavior. Summarize related batch success/failure counts through the original notification owner without task-location or temporary-mute buttons. Preserve configured sound/directory feedback, active task windows and B5 success/power-action authority; empty, failed, removed or historical tasks never authorize actions.

## Acceptance
After `11-13-download-finishing-progress`, production intake/terminal observation tests prove one summary for a settled batch, independent overlapping groups, explicit failures, no replay on restart and unchanged standalone handling. Both grouped and standalone completion notices have no business actions and leave permanent settings unchanged. Existing official notification clock/stack remains the only presentation owner.
