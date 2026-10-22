namespace NxTools.PlatformHost.Tests;

internal static class TestSelectionTests
{
    internal static Task DefaultDoesNotSelectNativeAcceptance()
    {
        var selected = TestSelection.Select(Program.Catalog(), Array.Empty<string>());
        Require(selected.Length > 0 && selected.All(test => test.Effect == TestEffect.Fixture), "Default verification must select only fixtures.");
        return Task.CompletedTask;
    }

    internal static Task ZeroMatchIsAnError()
    {
        Reject(() => TestSelection.Select(Program.Catalog(), new[] { "definitely-no-such-test-2bf9" }));
        return Task.CompletedTask;
    }

    internal static Task NativeNeedsAuthorizationAndExplicitFilter()
    {
        var cases = new[] { new TestCase("native sample", () => throw new Exception("must not execute"), TestEffect.NativeAcceptance, "synthetic selection only") };
        Reject(() => TestSelection.Select(cases, new[] { "native sample" }));
        Reject(() => TestSelection.Select(cases, new[] { "--native", "native sample" }));
        Reject(() => TestSelection.Select(cases, new[] { "--native", "--authorize-native" }));
        var selected = TestSelection.Select(cases, new[] { "--native", "--authorize-native", "native sample" });
        Require(selected.Length == 1, "Explicit native selection remains available.");
        Require(TestSelection.Select(cases, new[] { "--native", "--list" }).Length == 1, "Listing native acceptance needs no execution authorization.");
        return Task.CompletedTask;
    }

    internal static Task LegacyFiltersKeepUnionAndCatalogOrder()
    {
        static Task Noop() => Task.CompletedTask;
        var cases = new[] {
            new TestCase("Alpha fixture", Noop, TestEffect.Fixture, "pure"),
            new TestCase("Beta fixture", Noop, TestEffect.Fixture, "pure"),
            new TestCase("Gamma acceptance", Noop, TestEffect.NativeAcceptance, "live"),
        };
        var selected = TestSelection.Select(cases, new[] { "BETA", "alpha", "alpha" });
        Require(selected.Select(test => test.Name).SequenceEqual(new[] { "Alpha fixture", "Beta fixture" }), "Legacy substring union/order/dedup must stay compatible.");
        Reject(() => TestSelection.Select(cases, new[] { "fixture", "acceptance" }));
        Reject(() => TestSelection.Select(cases, new[] { "--unknown" }));
        Reject(() => TestSelection.Select(cases, new[] { "--fixtures", "--native", "--list" }));
        return Task.CompletedTask;
    }

    internal static Task CatalogRetainsAcceptanceAndExplicitScopes()
    {
        var catalog = Program.Catalog();
        Require(catalog.Select(test => test.Name).Distinct().Count() == catalog.Length, "Test names unique.");
        Require(catalog.All(test => !string.IsNullOrWhiteSpace(test.Scope) && !test.Scope.Contains("pending", StringComparison.OrdinalIgnoreCase)), "Every entry must have reviewed effect scope.");
        foreach (var name in new[] {
            "real Directory Opus lister exposes a third-party navigation context",
            "real Directory Opus navigation reaches C drive targets",
            "real Files window exposes a third-party navigation context",
            "real Explorer Shell window resolves its exact HWND and self-navigates",
        })
        {
            Require(catalog.Single(test => test.Name == name).Effect != TestEffect.Fixture, "Live/environment-affecting test is retained outside fixture scope: " + name);
        }
        Require(catalog.Single(test => test.Name == "browser extension packages produce crx and xpi artifacts").Effect == TestEffect.Fixture, "CRX test must use isolated explicit PEM and remain a fixture");
        Reject(() => TestEffects.Create("unreviewed", Unreviewed));
        return Task.CompletedTask;
    }

    internal static async Task RunnerCountsSkipWithoutPassAndFailsFast()
    {
        using var output = new StringWriter();
        using var errors = new StringWriter();
        var cases = new[] {
            new TestCase("pass", () => Task.CompletedTask, TestEffect.Fixture, "pure"),
            new TestCase("skip", () => throw new TestSkippedException("synthetic unavailable"), TestEffect.Fixture, "pure"),
            new TestCase("fail", () => throw new InvalidOperationException("synthetic failure"), TestEffect.Fixture, "pure"),
            new TestCase("not reached", () => throw new InvalidOperationException("must not run"), TestEffect.Fixture, "pure"),
        };
        Require(await TestRunner.Run(cases, output, errors) == 1, "Failures keep nonzero exit.");
        var text = output.ToString();
        Require(text.Contains("SUMMARY selected=4 run=3 pass=1 skip=1 fail=1 not_run=1"), "Counts reconcile exactly.");
        Require(!text.Contains("PASS skip") && !text.Contains("RUN not reached"), "Skip is not PASS; fail-fast retained.");
        Require(errors.ToString().Contains("FAIL fail"), "Failure is explicit.");
    }

    private static Task Unreviewed() => Task.CompletedTask;

    private static void Reject(Action action)
    {
        try { action(); }
        catch (ArgumentException) { return; }
        throw new InvalidOperationException("Unsafe/invalid selection must reject before execution.");
    }
    private static void Require(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
