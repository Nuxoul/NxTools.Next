using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Globalization;
using System.Runtime.InteropServices;
using System.Text.RegularExpressions;
using System.Threading;
using System.Windows.Forms;

// Test-only, in-process observation. Read returns absolute CPU time, not CPU percent.
// Status "available" includes measured zero; every other status has a null scalar.
public sealed class NativeMaterialBenchmarkSample
{
    public int ProcessId { get; set; }
    public string ProcessStatus { get; set; }
    public int? ProcessNativeCode { get; set; }
    public string MemoryStatus { get; set; }
    public double? CpuTotalMilliseconds { get; set; }
    public long? PrivateBytes { get; set; }
    public long? WorkingSetBytes { get; set; }
    public int? HandleCount { get; set; }
    public string GpuEngineStatus { get; set; }
    public uint? GpuEngineNativeCode { get; set; }
    public double? GpuMaxEnginePercent { get; set; }
    public Dictionary<string, double> GpuEnginePercent { get; set; }
    public string GpuDedicatedMemoryStatus { get; set; }
    public uint? GpuDedicatedMemoryNativeCode { get; set; }
    public double? GpuDedicatedBytes { get; set; }
    public string GpuSharedMemoryStatus { get; set; }
    public uint? GpuSharedMemoryNativeCode { get; set; }
    public double? GpuSharedBytes { get; set; }
}

// The pure aggregation seam consumes the same formatted PDH records as the native reader.
public sealed class NativeMaterialBenchmarkCounterValue
{
    public string InstanceName { get; set; }
    public uint Status { get; set; }
    public double Value { get; set; }
}

public sealed class NativeMaterialBenchmarkCounterBatch
{
    public string Status { get; set; }
    public uint? NativeCode { get; set; }
    public NativeMaterialBenchmarkCounterValue[] Items { get; set; }
}

public static class NativeMaterialBenchmarkGpuAggregation
{
    private static readonly Regex Instance = new Regex(
        @"^pid_(\d+)_(luid_0x[0-9a-f]+_0x[0-9a-f]+_phys_\d+)(?:_eng_(\d+)_engtype_[^#]+)?(?:#\d+)?$",
        RegexOptions.IgnoreCase | RegexOptions.CultureInvariant);

    internal static HashSet<int> Select(int[] processIds)
    {
        if (processIds == null) throw new ArgumentNullException("processIds");
        if (processIds.Length > 128) throw new ArgumentOutOfRangeException("processIds", "At most 128 requested IDs.");
        HashSet<int> selected = new HashSet<int>();
        foreach (int id in processIds)
        {
            if (id <= 0) throw new ArgumentOutOfRangeException("processIds");
            selected.Add(id);
        }
        return selected;
    }

    internal static bool Parse(string name, out int id, out string engine)
    {
        id = 0;
        engine = null;
        if (name == null) return false;
        Match match = Instance.Match(name);
        if (!match.Success || !int.TryParse(match.Groups[1].Value, NumberStyles.None,
            CultureInfo.InvariantCulture, out id)) return false;
        if (match.Groups[3].Success)
            engine = (match.Groups[2].Value + "_eng_" + match.Groups[3].Value).ToLowerInvariant();
        return true;
    }

    public static NativeMaterialBenchmarkSample[] Aggregate(int[] processIds,
        NativeMaterialBenchmarkCounterBatch engines, NativeMaterialBenchmarkCounterBatch dedicated,
        NativeMaterialBenchmarkCounterBatch shared)
    {
        HashSet<int> selected = Select(processIds);
        List<NativeMaterialBenchmarkSample> result = new List<NativeMaterialBenchmarkSample>();
        foreach (int id in processIds)
        {
            if (!selected.Remove(id)) continue;
            NativeMaterialBenchmarkSample sample = new NativeMaterialBenchmarkSample();
            sample.ProcessId = id;
            sample.ProcessStatus = "not_sampled";
            sample.GpuEnginePercent = new Dictionary<string, double>(StringComparer.Ordinal);
            string status;
            uint? code;
            Sum(id, engines, sample.GpuEnginePercent, out status, out code);
            sample.GpuEngineStatus = status;
            sample.GpuEngineNativeCode = code;
            if (status == "available")
            {
                double maximum = 0;
                foreach (double value in sample.GpuEnginePercent.Values) maximum = Math.Max(maximum, value);
                sample.GpuMaxEnginePercent = maximum;
            }
            sample.GpuDedicatedBytes = Sum(id, dedicated, null, out status, out code);
            sample.GpuDedicatedMemoryStatus = status;
            sample.GpuDedicatedMemoryNativeCode = code;
            sample.GpuSharedBytes = Sum(id, shared, null, out status, out code);
            sample.GpuSharedMemoryStatus = status;
            sample.GpuSharedMemoryNativeCode = code;
            result.Add(sample);
        }
        return result.ToArray();
    }

    private static double? Sum(int id, NativeMaterialBenchmarkCounterBatch batch,
        Dictionary<string, double> perEngine, out string status, out uint? code)
    {
        status = batch == null ? "counter_unavailable" : batch.Status;
        code = batch == null ? null : batch.NativeCode;
        if (status != "available") return null;
        bool found = false;
        bool invalid = false;
        double total = 0;
        foreach (NativeMaterialBenchmarkCounterValue item in batch.Items ?? new NativeMaterialBenchmarkCounterValue[0])
        {
            int itemId;
            string engine;
            if (!Parse(item.InstanceName, out itemId, out engine) || itemId != id ||
                (perEngine == null ? engine != null : engine == null)) continue;
            found = true;
            if (item.Status > 1 || double.IsNaN(item.Value) || double.IsInfinity(item.Value) || item.Value < 0)
            {
                invalid = true;
                if (!code.HasValue && item.Status > 1) code = item.Status;
                continue;
            }
            if (perEngine != null)
            {
                double previous;
                perEngine.TryGetValue(engine, out previous);
                perEngine[engine] = previous + item.Value;
            }
            total += item.Value;
        }
        status = invalid ? "invalid_data" : found ? "available" : "no_instance";
        if (invalid || !found) return null;
        return total;
    }
}

public sealed class NativeMaterialBenchmarkSampler : IDisposable
{
    private const uint MoreData = 0x800007D2;
    private const uint NoInstance = 0x800007D1;
    private const uint NoData = 0x800007D5;
    private const uint FormatDoubleNoCap = 0x00000200 | 0x00008000;
    private const int MaximumBufferBytes = 16 * 1024 * 1024;
    private readonly object gate = new object();
    private IntPtr query;
    private bool disposed;
    private bool pdhAvailable;
    private readonly Counter engines = new Counter();
    private readonly Counter dedicated = new Counter();
    private readonly Counter shared = new Counter();

    private sealed class Counter
    {
        internal IntPtr Handle;
        internal string Status = "counter_unavailable";
        internal uint? NativeCode;
    }

    // PDH_FMT_COUNTERVALUE's DWORD is followed by an 8-byte-aligned union.
    [StructLayout(LayoutKind.Explicit, Size = 16)]
    private struct FormattedValue
    {
        [FieldOffset(0)] public uint Status;
        [FieldOffset(8)] public double Value;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct CounterItem
    {
        public IntPtr Name;
        public FormattedValue Value;
    }

    [DllImport("pdh.dll", EntryPoint = "PdhOpenQueryW", CharSet = CharSet.Unicode)]
    private static extern uint OpenQuery(string source, IntPtr userData, out IntPtr queryHandle);
    [DllImport("pdh.dll", EntryPoint = "PdhAddEnglishCounterW", CharSet = CharSet.Unicode)]
    private static extern uint AddCounter(IntPtr queryHandle, string path, IntPtr userData, out IntPtr counter);
    [DllImport("pdh.dll", EntryPoint = "PdhCollectQueryData")]
    private static extern uint Collect(IntPtr queryHandle);
    [DllImport("pdh.dll", EntryPoint = "PdhGetFormattedCounterArrayW", CharSet = CharSet.Unicode)]
    private static extern uint GetArray(IntPtr counter, uint format, ref uint bytes, out uint count, IntPtr buffer);
    [DllImport("pdh.dll", EntryPoint = "PdhCloseQuery")]
    private static extern uint CloseQuery(IntPtr queryHandle);

    public NativeMaterialBenchmarkSampler()
    {
        try
        {
            uint status = OpenQuery(null, IntPtr.Zero, out query);
            if (status != 0)
            {
                SetUnavailable("query_unavailable", status);
                return;
            }
            Add(engines, @"\GPU Engine(*)\Utilization Percentage");
            Add(dedicated, @"\GPU Process Memory(*)\Dedicated Usage");
            Add(shared, @"\GPU Process Memory(*)\Shared Usage");
            // Rate counters need an initial collection before the parent's timed interval.
            Collect(query);
            pdhAvailable = true;
        }
        catch (DllNotFoundException) { SetUnavailable("pdh_unavailable", null); }
        catch (EntryPointNotFoundException) { SetUnavailable("pdh_unavailable", null); }
    }

    private void SetUnavailable(string status, uint? code)
    {
        foreach (Counter counter in new Counter[] { engines, dedicated, shared })
        {
            counter.Status = status;
            counter.NativeCode = code;
        }
    }

    private void Add(Counter counter, string path)
    {
        uint status = AddCounter(query, path, IntPtr.Zero, out counter.Handle);
        counter.NativeCode = status == 0 ? (uint?)null : status;
        counter.Status = status == 0 ? "available" : "counter_unavailable";
    }

    public NativeMaterialBenchmarkSample[] Read(int[] processIds)
    {
        HashSet<int> selected = NativeMaterialBenchmarkGpuAggregation.Select(processIds);
        lock (gate)
        {
            if (disposed) throw new ObjectDisposedException("NativeMaterialBenchmarkSampler");
            uint? collection = !pdhAvailable ? null : (uint?)Collect(query);
            NativeMaterialBenchmarkSample[] samples = NativeMaterialBenchmarkGpuAggregation.Aggregate(processIds,
                ReadCounter(engines, selected, collection), ReadCounter(dedicated, selected, collection),
                ReadCounter(shared, selected, collection));
            foreach (NativeMaterialBenchmarkSample sample in samples) ReadProcess(sample);
            return samples;
        }
    }

    private static NativeMaterialBenchmarkCounterBatch ReadCounter(Counter counter, HashSet<int> selected, uint? collection)
    {
        NativeMaterialBenchmarkCounterBatch batch = new NativeMaterialBenchmarkCounterBatch();
        batch.Status = counter.Status;
        batch.NativeCode = counter.NativeCode;
        if (counter.Status != "available") return batch;
        if (collection.HasValue && collection.Value != 0)
        {
            batch.Status = collection.Value == NoData ? "no_data" : "collect_error";
            batch.NativeCode = collection;
            return batch;
        }
        uint bytes = 0;
        uint count;
        uint status = GetArray(counter.Handle, FormatDoubleNoCap, ref bytes, out count, IntPtr.Zero);
        if (status != MoreData) return ArrayFailure(batch, status);
        // One bounded retry if instances change between size discovery and copying.
        for (int attempt = 0; attempt < 2; attempt++)
        {
            if (bytes == 0 || bytes > MaximumBufferBytes)
            {
                batch.Status = "buffer_limit";
                batch.NativeCode = null;
                return batch;
            }
            int capacity = (int)bytes;
            IntPtr buffer = Marshal.AllocHGlobal(capacity);
            try
            {
                status = GetArray(counter.Handle, FormatDoubleNoCap, ref bytes, out count, buffer);
                if (status == MoreData) continue;
                if (status != 0) return ArrayFailure(batch, status);
                int stride = Marshal.SizeOf(typeof(CounterItem));
                if (count > capacity / stride)
                {
                    batch.Status = "invalid_data";
                    return batch;
                }
                List<NativeMaterialBenchmarkCounterValue> items = new List<NativeMaterialBenchmarkCounterValue>();
                for (int index = 0; index < (int)count; index++)
                {
                    CounterItem item = (CounterItem)Marshal.PtrToStructure(IntPtr.Add(buffer, index * stride), typeof(CounterItem));
                    string name = Marshal.PtrToStringUni(item.Name);
                    int id;
                    string engine;
                    if (!NativeMaterialBenchmarkGpuAggregation.Parse(name, out id, out engine) || !selected.Contains(id)) continue;
                    items.Add(new NativeMaterialBenchmarkCounterValue { InstanceName = name, Status = item.Value.Status, Value = item.Value.Value });
                }
                batch.Items = items.ToArray();
                return batch;
            }
            finally { Marshal.FreeHGlobal(buffer); }
        }
        return ArrayFailure(batch, status);
    }

    private static NativeMaterialBenchmarkCounterBatch ArrayFailure(NativeMaterialBenchmarkCounterBatch batch, uint status)
    {
        batch.Status = status == NoInstance ? "no_instance" : status == NoData ? "no_data" : status == 0 ? "no_instance" : "read_error";
        batch.NativeCode = status == 0 ? (uint?)null : status;
        return batch;
    }

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr OpenProcess(uint access, bool inherit, int processId);
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool GetProcessTimes(IntPtr process, out long created, out long exited, out long kernel, out long user);
    [DllImport("kernel32.dll")]
    private static extern bool CloseHandle(IntPtr handle);

    private static void ReadProcess(NativeMaterialBenchmarkSample sample)
    {
        try
        {
            using (Process process = Process.GetProcessById(sample.ProcessId))
            {
                process.Refresh();
                IntPtr limited = OpenProcess(0x1000, false, sample.ProcessId);
                if (limited == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
                try
                {
                    long created, exited, kernel, user;
                    if (!GetProcessTimes(limited, out created, out exited, out kernel, out user))
                        throw new Win32Exception(Marshal.GetLastWin32Error());
                    sample.CpuTotalMilliseconds = (kernel + user) / 10000.0;
                    sample.ProcessStatus = "available";
                }
                finally { CloseHandle(limited); }
                try
                {
                    sample.PrivateBytes = process.PrivateMemorySize64;
                    sample.WorkingSetBytes = process.WorkingSet64;
                    sample.HandleCount = process.HandleCount;
                    sample.MemoryStatus = "available";
                }
                catch (Win32Exception ex)
                {
                    sample.MemoryStatus = ex.NativeErrorCode == 5 ? "access_denied" : "read_error";
                    sample.ProcessNativeCode = ex.NativeErrorCode;
                }
            }
        }
        catch (ArgumentException) { sample.ProcessStatus = "missing"; }
        catch (InvalidOperationException) { sample.ProcessStatus = "missing"; }
        catch (Win32Exception ex)
        {
            sample.ProcessStatus = ex.NativeErrorCode == 5 ? "access_denied" : "read_error";
            sample.ProcessNativeCode = ex.NativeErrorCode;
        }
    }

    public void Dispose()
    {
        lock (gate)
        {
            if (disposed) return;
            disposed = true;
            if (query != IntPtr.Zero) CloseQuery(query);
            query = IntPtr.Zero;
        }
        GC.SuppressFinalize(this);
    }

    ~NativeMaterialBenchmarkSampler() { Dispose(); }
}

// One synthetic, non-topmost, nonactivating window. Start is opt-in; construction
// creates no HWND/timer/bitmap. The parent starts its foreground benchmark afterward.
public sealed class NativeMaterialBenchmarkBackdrop : IDisposable
{
    private readonly object gate = new object();
    private readonly ManualResetEventSlim ready = new ManualResetEventSlim(false);
    private Thread thread;
    private BackdropForm form;
    private Exception startupFailure;
    private bool disposed;
    private volatile bool stopRequested;
    private volatile bool moving;
    private long windowHandle;
    private long paints;
    private long updates;

    public NativeMaterialBenchmarkBackdrop()
    {
        using (Process process = Process.GetCurrentProcess()) ProcessId = process.Id;
    }

    public int ProcessId { get; private set; }
    public long WindowHandle { get { return Interlocked.Read(ref windowHandle); } }
    public bool IsMoving { get { return moving; } }
    public long PaintCount { get { return Interlocked.Read(ref paints); } }
    public long UpdateCount { get { return Interlocked.Read(ref updates); } }

    public void Start()
    {
        lock (gate)
        {
            if (disposed) throw new ObjectDisposedException("NativeMaterialBenchmarkBackdrop");
            if (thread != null) throw new InvalidOperationException("Backdrop already started.");
            thread = new Thread(Run);
            thread.IsBackground = true;
            thread.Name = "Native material benchmark backdrop";
            thread.SetApartmentState(ApartmentState.STA);
            thread.Start();
        }
        if (!ready.Wait(10000))
        {
            Dispose();
            throw new TimeoutException("Backdrop readiness exceeded 10 seconds.");
        }
        if (startupFailure != null)
        {
            Dispose();
            throw new InvalidOperationException("Backdrop startup failed.", startupFailure);
        }
    }

    private void Run()
    {
        try
        {
            using (BackdropForm ownForm = new BackdropForm(this))
            {
                lock (gate) form = ownForm;
                ownForm.Shown += delegate
                {
                    Interlocked.Exchange(ref windowHandle, ownForm.Handle.ToInt64());
                    ready.Set();
                    if (stopRequested) ownForm.Close();
                };
                if (!stopRequested) Application.Run(ownForm);
            }
        }
        catch (Exception ex) { startupFailure = ex; }
        finally
        {
            moving = false;
            Interlocked.Exchange(ref windowHandle, 0);
            lock (gate) form = null;
            ready.Set();
        }
    }

    public void SetMoving(bool value)
    {
        lock (gate)
        {
            if (disposed) throw new ObjectDisposedException("NativeMaterialBenchmarkBackdrop");
            if (form == null || WindowHandle == 0) throw new InvalidOperationException("Backdrop is not ready.");
            BackdropForm ownForm = form;
            ownForm.BeginInvoke((MethodInvoker)delegate { if (!ownForm.IsDisposed) ownForm.SetMoving(value); });
        }
    }

    public void Dispose()
    {
        Thread ownThread;
        lock (gate)
        {
            disposed = true;
            stopRequested = true;
            ownThread = thread;
            if (form != null && WindowHandle != 0)
            {
                BackdropForm ownForm = form;
                try { ownForm.BeginInvoke((MethodInvoker)delegate { ownForm.Close(); }); }
                catch (InvalidOperationException) { /* The owned form has already closed. */ }
            }
        }
        if (ownThread != null && !ownThread.Join(10000))
            throw new TimeoutException("Backdrop thread did not stop within 10 seconds.");
        // No waiter or UI thread remains after the join.
        ready.Dispose();
    }

    private sealed class BackdropForm : Form
    {
        private readonly NativeMaterialBenchmarkBackdrop owner;
        private readonly Bitmap background;
        private readonly System.Windows.Forms.Timer timer;
        private int offset;

        internal BackdropForm(NativeMaterialBenchmarkBackdrop owner)
        {
            this.owner = owner;
            Text = "Native material benchmark synthetic backdrop";
            FormBorderStyle = FormBorderStyle.None;
            StartPosition = FormStartPosition.Manual;
            Bounds = Screen.PrimaryScreen.WorkingArea;
            ShowInTaskbar = false;
            TopMost = false;
            DoubleBuffered = true;
            background = CreateBackground(ClientSize.Width + 128, ClientSize.Height);
            timer = new System.Windows.Forms.Timer();
            timer.Interval = 16;
            timer.Tick += delegate
            {
                offset = (offset + 4) % 128;
                Interlocked.Increment(ref owner.updates);
                Invalidate();
            };
        }

        protected override bool ShowWithoutActivation { get { return true; } }

        internal void SetMoving(bool value)
        {
            timer.Enabled = value;
            owner.moving = value;
            if (!value) offset = 0;
            Invalidate();
        }

        private static Bitmap CreateBackground(int width, int height)
        {
            Bitmap bitmap = new Bitmap(width, height);
            using (Graphics graphics = Graphics.FromImage(bitmap))
            using (LinearGradientBrush gradient = new LinearGradientBrush(new Rectangle(0, 0, width, height),
                Color.FromArgb(108, 158, 200), Color.FromArgb(190, 104, 132), 35f))
            using (SolidBrush light = new SolidBrush(Color.FromArgb(235, 239, 248)))
            using (SolidBrush dark = new SolidBrush(Color.FromArgb(32, 42, 65)))
            {
                graphics.FillRectangle(gradient, 0, 0, width, height);
                for (int x = 0; x < width; x += 32)
                    graphics.FillRectangle((x / 32) % 2 == 0 ? light : dark, x, height / 3, 16, height / 5);
                for (int y = height / 2; y < height; y += 32)
                    graphics.FillRectangle((y / 32) % 2 == 0 ? light : dark, 0, y, width, 4);
                graphics.FillRectangle(light, width / 10, height / 10, width / 5, height / 8);
                graphics.FillRectangle(dark, width * 7 / 10, height * 3 / 4, width / 5, height / 8);
            }
            return bitmap;
        }

        protected override void OnPaint(PaintEventArgs e)
        {
            e.Graphics.DrawImageUnscaled(background, -offset, 0);
            Interlocked.Increment(ref owner.paints);
            base.OnPaint(e);
        }

        protected override void Dispose(bool disposing)
        {
            if (disposing)
            {
                timer.Stop();
                timer.Dispose();
                background.Dispose();
                owner.moving = false;
            }
            base.Dispose(disposing);
        }
    }
}
