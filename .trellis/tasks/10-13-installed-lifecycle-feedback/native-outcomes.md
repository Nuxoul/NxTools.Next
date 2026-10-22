# Explorer completion readback correction

## Pre-edit evidence
Read download/native-reference task outcomes and source before change. Rust matching-success/current-window close already exists; screenshot symptom can result from a real native operation followed by false-negative readback. ExplorerShellActivation.TryGetExplorerDirectory consumed a private IDispatch interface with IWebBrowserApp GUID0002DF05 and DISPIDs0x60020003/0x60020008, while ExplorerWindowAdapter's existing IExplorerBrowserDispatch uses IWebBrowser2 GUID D30C1661-CDAF-11D0-8A3E-00C04FC9E26E, HWND DispId-515/int and LocationURL DispId211/string. The former interface is incompatible with the intended Explorer dispatch properties.

## Executed red
Added reflection assertions to existing reviewed-safe InstalledNativeActionTests.ExplorerNativeFirstAndFallback. dotnet build succeeded0warnings/errors; exact fixture ran1failed1 at GUID contract before fake native action. No real Explorer/COM/Shell call.

## Fix boundary
Reuse existing IExplorerBrowserDispatch; remove wrong private duplicate only. Native select/open -> matching directory/window -> foreground confirmation -> correlated result -> Rust keyed close stays unchanged. Failed/cancelled/stale operations still retain window. Real installed Explorer behavior is not yet tested.

## Executed green
After removing the duplicate, test build0warnings/errors and4exact reviewed-safe InstalledNativeActionTests fixtures passed (Explorer native-first/fallback, readback/retry/pending, OpenWith cancellation/failure, PIDL lifetime), RUN4/PASS4/SKIP0/FAIL0. Tests use fake native operations/temp targets; no real COM enumeration or native acceptance.
