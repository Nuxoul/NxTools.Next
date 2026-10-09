# Window contracts

Read download/gpui reference notes and legacy AddDownloadWindow constructor/open/closed/size/move paths. Per-HWND requests need atomic generation ownership across apply/replay/retirement; final correction uses the same snapshot. Never hold a global lock across cross-process Windows calls. Bound or asynchronously marshal native operations and treat acceptance distinct from final size. Compact feedback does not require future native growth to be visible. Validation invalidates any former probe generation before asynchronous results can apply. Details layout uses native centered command when HWND exists; test fallback may use GPUI resize.

Parent owns desktop downloads and C# adapter/recovery/test files. No parallel editor on those files.
