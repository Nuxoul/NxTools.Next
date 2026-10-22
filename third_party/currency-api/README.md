# Currency API catalog data

Source: https://cdn.jsdelivr.net/npm/@fawazahmed0/currency-api@latest/v1/currencies.json
Project: https://github.com/fawazahmed0/exchange-api
License: CC0-1.0, original LICENSE-CC0 retained here.

Fetched2026-10-20; HTTP200;339code/name entries (includes fiat,metals,cryptocurrencies;some names empty). Raw JSON snapshot in rust/apps/nxtools-desktop/assets/currency-catalog.json, SHA25672c81a54212045b8a2cced272379591a2d1d8b6c2a28f9b9b94a0b02a3657617. This is a bootstrap catalog only,not exchange rates; live paired catalog/CNY rates replace it in disposable local cache after validation. Source text and numeric amounts from user's clipboard are never sent to provider.

Captured public CNY response for offline regression: rust/apps/nxtools-desktop/src/tools/currency-fixtures/cny-2026-10-19.json, reference date2026-10-19,339rates. This synthetic-test input is not loaded as runtime rates. Same CC0 source/license. Tests read immutable checked-in bytes, not session scratch/publicnetwork.
