# Sarasa UI SC private-font assessment

## Local files
TTF name/OS/2-table inspection of the user-provided `D:\Temp\SarasaUi-TTF-1.0.43` found ten SC files totaling244,278,404bytes (232.96MiB). The approved bundle contains only unmodified Regular; no Windows font is installed.

| File | Weight | Raw bytes | MiB |
| --- | --- | --- | --- |
| SarasaUiSC-Regular.ttf | 400 | 24,049,996 | 22.94 |
| SarasaUiSC-SemiBold.ttf | 600 | 23,978,372 | 22.87 |
| SarasaUiSC-Bold.ttf | 700 | 23,896,256 | 22.79 |

The remaining files are Light300, ExtraLight200 and italic counterparts. Regular is the normal UI face; SemiBold fits emphasized UI labels; Bold is optional for700-weight text. Italic/light faces need not be bundled for ordinary controls. Regular plus SemiBold adds48,028,368raw bytes (45.80MiB), rather than the full ten-face set. Installer compression changes download size, so these raw sizes are not a measured installer-size increase.

## Names and loading
Regular's name-table family is `Sarasa UI SC`; SemiBold's legacy family is `Sarasa UI SC SemiBold`, with typographic family `Sarasa UI SC` and style SemiBold. Use internal family/weight metadata rather than filenames, and verify actual600-weight selection if bundling.

Current Windows GPUI backend `vendor/gpui-pre-windows-039/src/direct_write.rs::add_fonts` uses DirectWrite's CreateInMemoryFontFileReference/AddFontFile/CreateFontSet/CreateFontCollectionFromFontSet. Application-private fonts are supported without installing a Windows font or changing other programs. Static borrowed font bytes can remain owned for the application lifetime. Existing user-selected family and missing-font fallback policy must be retained if this is implemented.

## License
The official repository LICENSE was retrieved read-only from `https://api.github.com/repos/be5invis/Sarasa-Gothic/license`, resolving to `https://github.com/be5invis/Sarasa-Gothic/blob/main/LICENSE`. Its text explicitly licenses the font under SIL Open Font License1.1. Bundling/embedding and redistribution with software are permitted with the copyright notice and full license. Fonts remain under OFL and cannot be sold by themselves. Modified/subset derivatives must observe the stated Reserved Font Name conditions. The local font copyright matches the upstream notice; local fsType is0, which is technical embedding metadata, not a substitute for the license.

## Approved bundle
The user selected only Regular as the app-private default `Sarasa UI SC`. Its24,049,996raw bytes (22.94MiB) and full copyright/OFL text are included; SemiBold, Bold, italic and light faces are not bundled. Explicit saved UI-font choices remain authoritative. Actual compressed installer growth is unmeasured. Implementation and acceptance boundaries are recorded in [the font task](../11-20-sarasa-ui-font/prd.md).
