# Bundled Sarasa UI SC Regular default

## Scope

User approves bundling only SarasaUiSC-Regular.ttf 1.0.43 and choosing its internal family `Sarasa UI SC` as the default UI font. The original unmodified 24,049,996-byte font is application-private, not installed in Windows. Include the copyright and complete SIL OFL 1.1 in source assets and the existing About license view.

## Implementation boundary

Core owns DEFAULT_UI_FONT_FAMILY and serde defaults. Desktop assets owns embedded static bytes and one startup registration through the released TextSystem::add_fonts(Vec<Cow<'static,[u8]>>) API. Register before applying theme/saved family and creating windows. Preserve explicit saved font intent, original atomic Apply/Cancel, font sizes, Consolas and theme colors. Existing official Medium32 Select and About scroll/TitleBar/buttons remain unchanged; only their data/content changes. No dependency/vendor/native API, installer script, config migration or system font registration.

Expected files: Core lib default/tests; Desktop assets and one startup call; About license data; one TTF plus OFL asset; global-font reference and appearance contract. Verify actual registration and font availability, default/explicit preference compatibility, resource/license content and original theme/settings behavior. Existing reference constructors/save/close owners in global-font-settings.md and legacy MainWindowViewModel/SettingsWindow are retained. Public DirectWrite in-memory collection is the private loading boundary.

## Acceptance

A red-capable Core regression detects the previous default. Embedded font is Regular400 with family Sarasa UI SC; exactly one Sarasa face is bundled. Startup registration precedes visible/theme use. Default selector names the bundled family, while explicit other choices survive configuration roundtrip. Existing production theme/paint/settings transaction and About tests remain passing. Serial Cargo uses workstation memory settings. No commit, Release packaging, installation, real config mutation or user visual acceptance is included.
