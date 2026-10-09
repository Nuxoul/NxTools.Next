# Browser extension web boundary

Evidence: `repo:browser_extension/nxtools-interceptor/manifest.json`, `repo:browser_extension/nxtools-interceptor/background.js`, `repo:browser_extension/nxtools-interceptor/popup.js`.

## Pre-Development Checklist
Read [authority](../shared/architecture.md), [engineering/privacy](../shared/engineering.md), [extension contracts](contracts.md), [UI/current gaps](ui-guidelines.md) and download reference before capture/handoff changes. MV3 worker/content/page-hook/popup are different trust/lifecycle contexts; browser item ownership remains browser until acceptance+cancel success.

## Quality Check
No package.json/npm test exists. From root PowerShell: `$extensionTests = (Get-ChildItem browser_extension/nxtools-interceptor/tests -Filter '*.test.js').FullName; node --test $extensionTests`. `node browser_extension/nxtools-interceptor/tests/blob-transfer-policy.test.js` runs direct assert fixture. Source assertions are not live browser UX/accessibility. Native Messaging gate and changed Rust host tests are related. No real browsing/cookie capture/install/native registry without authorization. [Entry](../index.md).
