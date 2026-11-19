using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.IO.Pipes;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using System.Windows.Automation;

namespace NxTools.QuickLaunchBenchmark
{
    public sealed class ResultRow
    {
        public string title;
        public string path;
        public double top;
    }

    public sealed class ExpectedTarget
    {
        public string name;
        public string parent_suffix;
    }

    public sealed class QuerySample
    {
        public string error;
        public double input_set_ms;
        public double? first_result_ms;
        public double? first_target_ms;
        public double? all_targets_ms;
        public double? last_result_change_ms;
        public int result_change_count;
        public int expected_target_count;
        public double observed_ms;
        public int polls;
        public double poll_time_ms;
        public double max_poll_ms;
        public bool focus_ok = true;
        public bool input_restored;
        public string observation_status = "deadline_snapshot";
        public ResultRow[] rows = new ResultRow[0];
    }

    public static class Native
    {
        private delegate bool EnumProc(IntPtr window, IntPtr parameter);
        [DllImport("user32.dll")] private static extern bool EnumWindows(EnumProc callback, IntPtr parameter);
        [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
        [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern IntPtr FindWindowExW(IntPtr parent, IntPtr after, string className, string title);
        [DllImport("user32.dll", SetLastError = true)] private static extern bool PostMessageW(IntPtr window, uint message, IntPtr word, IntPtr data);
        [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr window);
        [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
        [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);

        public static IntPtr[] OwnedWindows(uint processId)
        {
            var windows = new List<IntPtr>();
            EnumWindows(delegate(IntPtr window, IntPtr parameter) {
                uint owner;
                GetWindowThreadProcessId(window, out owner);
                if (owner == processId) windows.Add(window);
                return true;
            }, IntPtr.Zero);
            return windows.ToArray();
        }

        public static void ShowNxTools(uint platformProcessId)
        {
            var window = IntPtr.Zero;
            while ((window = FindWindowExW(new IntPtr(-3), window, null, "NxTools.PlatformHost.Events")) != IntPtr.Zero)
            {
                uint owner;
                GetWindowThreadProcessId(window, out owner);
                if (owner != platformProcessId) continue;
                if (!PostMessageW(window, 0x0312, new IntPtr(2), IntPtr.Zero))
                    throw new InvalidOperationException("NxTools summon native error " + Marshal.GetLastWin32Error());
                return;
            }
            throw new InvalidOperationException("The matching NxTools PlatformHost event window is unavailable.");
        }

        public static string ShowListary()
        {
            using (var pipe = new NamedPipeClientStream(".", "ListaryInternalNamedPipe", PipeDirection.InOut, PipeOptions.Asynchronous))
            {
                pipe.Connect(3000);
                using (var writer = new StreamWriter(pipe, new UTF8Encoding(false), 1024, true))
                using (var reader = new StreamReader(pipe, Encoding.UTF8, true, 1024, true))
                {
                    writer.AutoFlush = true;
                    writer.WriteLine("[url-schema]listary://show-launcher");
                    var response = reader.ReadLineAsync();
                    if (!response.Wait(5000)) throw new TimeoutException("Listary show-launcher response exceeded 5000 ms.");
                    if (response.Result != "1") throw new InvalidOperationException("Listary did not acknowledge show-launcher: " + response.Result);
                    return response.Result;
                }
            }
        }

        public static void Escape(IntPtr window)
        {
            if (GetForegroundWindow() != window) throw new InvalidOperationException("Cannot dismiss a launcher that is not foreground.");
            if (!PostMessageW(window, 0x0100, new IntPtr(0x1b), IntPtr.Zero))
                throw new InvalidOperationException("Launcher Escape native error " + Marshal.GetLastWin32Error());
            PostMessageW(window, 0x0101, new IntPtr(0x1b), IntPtr.Zero);
        }
    }

    public sealed class Launcher
    {
        public readonly string Product;
        public readonly IntPtr Window;
        private readonly AutomationElement root;
        private readonly ValuePattern input;

        private Launcher(string product, IntPtr window, AutomationElement rootElement, ValuePattern inputPattern)
        {
            Product = product;
            Window = window;
            root = rootElement;
            input = inputPattern;
        }

        public static Launcher Find(string product, uint processId)
        {
            var selector = product == "nxtools"
                ? new PropertyCondition(AutomationElement.NameProperty, "\u901f\u542f\u641c\u7d22")
                : new PropertyCondition(AutomationElement.AutomationIdProperty, "PART_SearchBox");
            foreach (var window in Native.OwnedWindows(processId))
            {
                if (!Native.IsWindowVisible(window)) continue;
                var candidate = AutomationElement.FromHandle(window);
                if (candidate.Current.ProcessId != processId) continue;
                if (product == "listary" && candidate.Current.Name != "LauncherSearchWindow") continue;
                var edit = candidate.FindFirst(TreeScope.Descendants, selector);
                if (edit == null) continue;
                object pattern;
                if (!edit.TryGetCurrentPattern(ValuePattern.Pattern, out pattern)) continue;
                return new Launcher(product, window, candidate, (ValuePattern)pattern);
            }
            return null;
        }

        public string InputValue { get { return input.Current.Value; } }
        public bool Foreground { get { return Native.IsWindowVisible(Window) && Native.GetForegroundWindow() == Window; } }

        public static ResultRow ParseNxLabel(string label, double top)
        {
            int separator = label.IndexOf('\uff0c');
            if (separator < 1) return null;
            return new ResultRow { title = label.Substring(0, separator), path = label.Substring(separator + 1), top = top };
        }

        public static bool IsListaryNavigation(string label)
        {
            return label == "Listary.Core.Search.Class177";
        }

        public ResultRow[] ReadRows()
        {
            var type = Product == "nxtools" ? ControlType.Button : ControlType.ListItem;
            var elements = root.FindAll(TreeScope.Descendants, new PropertyCondition(AutomationElement.ControlTypeProperty, type));
            var rows = new List<ResultRow>();
            for (int index = 0; index < elements.Count; index++)
            {
                var element = elements[index];
                var info = element.Current;
                if (info.IsOffscreen || !info.IsEnabled || info.BoundingRectangle.IsEmpty) continue;
                if (Product == "nxtools")
                {
                    var row = ParseNxLabel(info.Name, info.BoundingRectangle.Top);
                    if (row != null) rows.Add(row);
                    continue;
                }
                if (IsListaryNavigation(info.Name)) continue;
                var descendants = element.FindAll(TreeScope.Descendants, Condition.TrueCondition);
                string title = null;
                string parent = "";
                for (int childIndex = 0; childIndex < descendants.Count; childIndex++)
                {
                    var child = descendants[childIndex].Current;
                    if (child.IsOffscreen || String.IsNullOrEmpty(child.Name)) continue;
                    if (title == null && child.ControlType == ControlType.Text) title = child.Name;
                    if (child.ControlType == ControlType.Custom &&
                        ((child.Name.Length > 2 && child.Name[1] == ':') || child.Name.StartsWith("\\\\"))) parent = child.Name;
                }
                if (!String.IsNullOrEmpty(title)) rows.Add(new ResultRow { title = title, path = parent, top = info.BoundingRectangle.Top });
            }
            rows.Sort(delegate(ResultRow left, ResultRow right) { return left.top.CompareTo(right.top); });
            return rows.ToArray();
        }

        private static bool SameRows(ResultRow[] left, ResultRow[] right)
        {
            if (left.Length != right.Length) return false;
            for (int index = 0; index < left.Length; index++)
                if (left[index].title != right[index].title || left[index].path != right[index].path) return false;
            return true;
        }

        private static bool TargetSeen(ResultRow[] rows, ExpectedTarget target)
        {
            foreach (var row in rows)
            {
                if (!String.Equals(row.title, target.name, StringComparison.OrdinalIgnoreCase)) continue;
                if (String.IsNullOrEmpty(target.parent_suffix)) return true;
                string path = row.path.Replace('/', '\\').TrimEnd('\\');
                string parent = String.Equals(Path.GetFileName(path), target.name, StringComparison.OrdinalIgnoreCase) ? Path.GetDirectoryName(path) : path;
                if (String.Equals(parent, target.parent_suffix, StringComparison.OrdinalIgnoreCase) ||
                    parent.EndsWith("\\" + target.parent_suffix, StringComparison.OrdinalIgnoreCase)) return true;
            }
            return false;
        }

        private static void Observe(QuerySample sample, ResultRow[] rows, ExpectedTarget[] targets, double elapsedMs)
        {
            if (!sample.first_result_ms.HasValue && rows.Length > 0) sample.first_result_ms = elapsedMs;
            int found = 0;
            foreach (var target in targets) if (TargetSeen(rows, target)) found++;
            if (found > 0 && !sample.first_target_ms.HasValue) sample.first_target_ms = elapsedMs;
            if (targets.Length > 0 && found == targets.Length && !sample.all_targets_ms.HasValue) sample.all_targets_ms = elapsedMs;
            if (!SameRows(sample.rows, rows))
            {
                sample.rows = rows;
                sample.last_result_change_ms = elapsedMs;
                sample.result_change_count++;
            }
        }

        private void WaitForValue(string expected, string previous)
        {
            var clock = Stopwatch.StartNew();
            while (clock.ElapsedMilliseconds < 1500)
            {
                if (!Foreground) throw new InvalidOperationException("Launcher lost foreground while applying input.");
                string value = InputValue;
                if (value == expected) return;
                if (value != previous) throw new InvalidOperationException("Operator changed input while the UIA update was pending.");
                Thread.Sleep(10);
            }
            throw new TimeoutException("UIA input value did not acknowledge the requested update within 1500 ms.");
        }

        public void RequireEmpty()
        {
            if (InputValue != "") throw new InvalidOperationException("The launcher contains operator input; it was left untouched.");
            var clock = Stopwatch.StartNew();
            double? emptySince = null;
            while (clock.ElapsedMilliseconds < 1500)
            {
                if (!Foreground) throw new InvalidOperationException("Launcher lost foreground before sampling.");
                if (InputValue != "") throw new InvalidOperationException("Operator input changed before sampling.");
                if (ReadRows().Length == 0)
                {
                    if (!emptySince.HasValue) emptySince = clock.Elapsed.TotalMilliseconds;
                    if (clock.Elapsed.TotalMilliseconds - emptySince.Value >= 60) return;
                }
                else emptySince = null;
                Thread.Sleep(20);
            }
            throw new InvalidOperationException("Empty input still exposes result rows; an isolated query cannot be measured.");
        }

        public QuerySample Query(string query, ExpectedTarget[] targets, int observationMs, int quietMs, int timeoutMs, int pollMs)
        {
            RequireEmpty();
            var sample = new QuerySample { expected_target_count = targets.Length };
            var clock = Stopwatch.StartNew();
            double lastChanged = 0;
            try
            {
                input.SetValue(query);
                sample.input_set_ms = clock.Elapsed.TotalMilliseconds;
                WaitForValue(query, "");
                do
                {
                    if (!Foreground) { sample.focus_ok = false; throw new InvalidOperationException("Launcher lost foreground during sampling."); }
                    if (InputValue != query) throw new InvalidOperationException("Operator changed the query during sampling.");
                    double pollStart = clock.Elapsed.TotalMilliseconds;
                    var rows = ReadRows();
                    double now = clock.Elapsed.TotalMilliseconds;
                    double pollCost = now - pollStart;
                    sample.poll_time_ms += pollCost;
                    sample.max_poll_ms = Math.Max(sample.max_poll_ms, pollCost);
                    sample.polls++;
                    Observe(sample, rows, targets, now);
                    lastChanged = sample.last_result_change_ms ?? 0;
                    if (now >= observationMs && now - lastChanged >= quietMs)
                    {
                        sample.observation_status = "quiet_snapshot";
                        break;
                    }
                    Thread.Sleep(pollMs);
                } while (clock.ElapsedMilliseconds < timeoutMs);
            }
            catch (Exception exception) { sample.error = exception.Message; sample.focus_ok = Foreground; }
            finally
            {
                sample.observed_ms = clock.Elapsed.TotalMilliseconds;
                try
                {
                    if (InputValue == query)
                    {
                        input.SetValue("");
                        WaitForValue("", query);
                        RequireEmpty();
                        sample.input_restored = true;
                    }
                    else if (sample.error == null) sample.error = "Query no longer belongs to the benchmark; input was left untouched.";
                }
                catch (Exception exception)
                {
                    sample.input_restored = false;
                    if (sample.error == null) sample.error = "Query cleanup failed: " + exception.Message;
                }
            }
            return sample;
        }

        public static void SelfTest()
        {
            var parsed = ParseNxLabel("nxqlb-\u5e74\u5ea6\u62a5\u544a.txt\uff0cD:\\Fixture\\nxqlb-\u5e74\u5ea6\u62a5\u544a.txt", 12);
            if (parsed == null || parsed.title != "nxqlb-\u5e74\u5ea6\u62a5\u544a.txt" || parsed.path != "D:\\Fixture\\nxqlb-\u5e74\u5ea6\u62a5\u544a.txt")
                throw new InvalidOperationException("NxTools accessibility label parsing failed.");
            if (ParseNxLabel("\u6700\u8fd1\u76ee\u5f55", 0) != null) throw new InvalidOperationException("Header was counted as a result.");
            if (!IsListaryNavigation("Listary.Core.Search.Class177") || IsListaryNavigation("PathSearchItemViewModel [Class177.txt]"))
                throw new InvalidOperationException("Listary more-results classification failed.");
            if (SameRows(new ResultRow[] { parsed }, new ResultRow[0])) throw new InvalidOperationException("Result clearing comparison failed.");
            if (!SameRows(new ResultRow[] { parsed }, new ResultRow[] { new ResultRow { title = parsed.title, path = parsed.path, top = 100 } }))
                throw new InvalidOperationException("Visual row motion must not reset content settling.");
            var targets = new ExpectedTarget[] {
                new ExpectedTarget { name = "same.txt", parent_suffix = "team-a" },
                new ExpectedTarget { name = "same.txt", parent_suffix = "team-b" }
            };
            var timing = new QuerySample();
            Observe(timing, new ResultRow[0], targets, 10);
            if (timing.first_result_ms.HasValue || timing.last_result_change_ms.HasValue)
                throw new InvalidOperationException("Empty observation fabricated a response time.");
            var noise = new ResultRow { title = "other.txt", path = "D:\\Fixture\\other.txt" };
            Observe(timing, new ResultRow[] { noise }, targets, 20);
            if (timing.first_result_ms != 20 || timing.first_target_ms.HasValue)
                throw new InvalidOperationException("An unrelated first row was counted as the target.");
            var first = new ResultRow { title = "SAME.TXT", path = "D:\\Fixture\\team-a\\same.txt" };
            Observe(timing, new ResultRow[] { noise, first }, targets, 50);
            if (timing.first_target_ms != 50 || timing.all_targets_ms.HasValue)
                throw new InvalidOperationException("Target arrival ignored identity or parent disambiguation.");
            var second = new ResultRow { title = "same.txt", path = "D:\\Fixture\\team-b" };
            Observe(timing, new ResultRow[] { first, second }, targets, 80);
            Observe(timing, new ResultRow[] { first, second }, targets, 110);
            if (timing.all_targets_ms != 80 || timing.last_result_change_ms != 80 || timing.result_change_count != 3)
                throw new InvalidOperationException("Complete-target or last-change timing was overwritten by a quiet poll.");
        }
    }
}
