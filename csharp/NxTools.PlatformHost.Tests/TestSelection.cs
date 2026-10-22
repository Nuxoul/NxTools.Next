namespace NxTools.PlatformHost.Tests;

internal enum TestEffect { Fixture, NativeAcceptance, RestrictedFixture }
internal sealed record TestCase(string Name, Func<Task> Run, TestEffect Effect, string Scope);
internal sealed record TestRunOptions(bool Native, bool Authorized, bool List, string[] Filters)
{
    internal static TestRunOptions Parse(string[] args)
    {
        var native = false;
        var authorized = false;
        var list = false;
        var fixtures = false;
        var filters = new List<string>();
        foreach (var arg in args)
        {
            switch (arg)
            {
                case "--fixtures": fixtures = true; break;
                case "--native": native = true; break;
                case "--authorize-native": authorized = true; break;
                case "--list": list = true; break;
                default:
                    if (arg.StartsWith("--", StringComparison.Ordinal) || string.IsNullOrWhiteSpace(arg))
                        throw new ArgumentException($"Invalid test argument: {arg}");
                    filters.Add(arg);
                    break;
            }
        }
        if (native && fixtures) throw new ArgumentException("Choose --fixtures or --native, not both.");
        if (authorized && !native) throw new ArgumentException("--authorize-native requires --native.");
        if (native && !list && (!authorized || filters.Count == 0))
            throw new ArgumentException("Native acceptance requires --authorize-native and explicit name filters.");
        return new(native, authorized, list, filters.ToArray());
    }
}

internal static class TestSelection
{
    internal static TestCase[] Select(IReadOnlyList<TestCase> tests, string[] args) =>
        Select(tests, TestRunOptions.Parse(args));

    internal static TestCase[] Select(IReadOnlyList<TestCase> tests, TestRunOptions options)
    {
        bool InScope(TestCase test) => options.Native ? test.Effect != TestEffect.Fixture : test.Effect == TestEffect.Fixture;
        var matches = options.Filters.Length == 0
            ? tests.Where(InScope).ToArray()
            : tests.Where(test => options.Filters.Any(filter =>
                test.Name.Contains(filter, StringComparison.OrdinalIgnoreCase))).ToArray();
        if (matches.Length == 0) throw new ArgumentException("No tests match the requested selection.");
        var outside = matches.Where(test => !InScope(test)).ToArray();
        if (outside.Length != 0)
            throw new ArgumentException("Selection crosses the requested effect scope: " +
                string.Join(", ", outside.Select(test => test.Name)) +
                ". Use --list to inspect; native execution requires --native --authorize-native and explicit filters.");
        return matches;
    }
}

internal sealed class TestSkippedException(string reason) : Exception(reason);

internal static class TestRunner
{
    internal static async Task<int> Run(TestCase[] selected, TextWriter output, TextWriter errors)
    {
        var run = 0;
        var passed = 0;
        var skipped = 0;
        var failed = 0;
        foreach (var test in selected)
        {
            // Owned WinForms fixtures can install a context; this console has no pump.
            SynchronizationContext.SetSynchronizationContext(null);
            output.WriteLine($"RUN {test.Name}");
            run++;
            try
            {
                await test.Run();
                passed++;
                output.WriteLine($"PASS {test.Name}");
            }
            catch (TestSkippedException skip)
            {
                skipped++;
                output.WriteLine($"SKIP {test.Name}: {skip.Message}");
            }
            catch (Exception error)
            {
                failed++;
                errors.WriteLine($"FAIL {test.Name}: {error.Message}");
                break; // Preserve the existing fail-fast policy.
            }
        }
        output.WriteLine($"SUMMARY selected={selected.Length} run={run} pass={passed} skip={skipped} fail={failed} not_run={selected.Length - run}");
        return failed == 0 ? 0 : 1;
    }
}
