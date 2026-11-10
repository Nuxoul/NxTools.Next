using System.Collections.Concurrent;
using System.Diagnostics;
using System.Text;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

internal static class ArchiveExtractionWorkerTests
{
    private static void Check(bool condition, string detail) { if (!condition) throw new InvalidOperationException(detail); }
    public static async Task ActualZipToHandleSession()
    {
        var source=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtures=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        if (string.IsNullOrEmpty(source)||!File.Exists(source)||string.IsNullOrEmpty(fixtures)) throw new TestSkippedException("Explicit built Rust worker and synthetic ZIP fixture directory required.");
        var installed=Path.Combine(AppContext.BaseDirectory,"NxTools.ArchiveWorker.exe");
        if(File.Exists(installed)) throw new InvalidOperationException("Fixture cannot overwrite an existing worker.");
        var owned=Directory.CreateTempSubdirectory("archive-extract-chain-");var created=false;
        try
        {
            File.Copy(source,installed,false);created=true;
            foreach(var (name,success,files) in new[]{("deflated.zip",true,3),("bad-crc.zip",false,2)})
            {
                var target=Directory.CreateDirectory(Path.Combine(owned.FullName,Guid.NewGuid().ToString("N")));
                var events=new ConcurrentQueue<ArchiveWorkerEvent>();using var runner=ArchiveWorkerRunner.ForPackage(events.Enqueue);
                var request=new ArchiveWorkerRequest(Guid.NewGuid(),"extract",Path.Combine(fixtures,name),0,new {max_entries=100},OutputRoot:target.FullName);
                if(!runner.Start(request)) throw new InvalidOperationException("actual extraction admission");
                var clock=Stopwatch.StartNew();while(!events.Any(e=>e.Kind=="exited")){if(clock.ElapsedMilliseconds>15000)throw new InvalidOperationException("actual extraction deadline");await Task.Delay(20);}
                var terminal=events.Last();
                if(terminal.Category!=(success?"success":"worker_failure")||!terminal.CleanupComplete)throw new InvalidOperationException("actual extraction result: "+terminal.Category);
                if(terminal.FilesCommitted!=files || terminal.Journal?.Count(f=>f.Published==true)!=files)
                    throw new InvalidOperationException("CRC failure must retain only completed output journal: "+terminal.FilesCommitted);
                if(Directory.GetFiles(target.FullName,"*",SearchOption.AllDirectories).Length!=files)throw new InvalidOperationException("partial leaked or final output missing");
                if(File.ReadAllText(Path.Combine(target.FullName,"资料","说明.txt"))!="你好，压缩包\n")throw new InvalidOperationException("Chinese payload differs");
                if(success&&!File.ReadAllBytes(Path.Combine(target.FullName,"bytes.bin")).SequenceEqual(Enumerable.Range(0,256).Select(i=>(byte)i)))throw new InvalidOperationException("binary payload differs");
                if(!success && (File.Exists(Path.Combine(target.FullName,"bytes.bin")) || !File.Exists(Path.Combine(target.FullName,"empty"))))
                    throw new InvalidOperationException("bad CRC current file must be deleted while earlier closed files remain");
                if(events.Any(e=>e.Frame is {} frame&&frame.GetProperty("op").GetString()=="native_call"))throw new InvalidOperationException("file content escaped private output pump");
                if(!await runner.DrainAsync())throw new InvalidOperationException("actual extraction did not drain");
            }
        }
        finally{if(created)File.Delete(installed);owned.Delete(true);}
    }
    public static async Task CancelWhileSecondFileWriting()
    {
        var source = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (string.IsNullOrEmpty(source) || !File.Exists(source))
            throw new TestSkippedException("Explicit matching isolated worker required.");
        var installed = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        if (File.Exists(installed)) throw new InvalidOperationException("Fixture cannot replace an existing worker.");
        var owned = Directory.CreateTempSubdirectory("archive-direct-cancel-"); var copied = false;
        try
        {
            File.Copy(source, installed, false); copied = true;
            var archive = Path.Combine(owned.FullName, "two.zip");
            var target = Directory.CreateDirectory(Path.Combine(owned.FullName, "output"));
            using (var zip = System.IO.Compression.ZipFile.Open(archive, System.IO.Compression.ZipArchiveMode.Create))
            {
                using (var first = zip.CreateEntry("first.txt", System.IO.Compression.CompressionLevel.NoCompression).Open())
                    first.Write(Encoding.UTF8.GetBytes("first-completed"));
                using (var second = zip.CreateEntry("second.bin", System.IO.Compression.CompressionLevel.NoCompression).Open())
                {
                    var block = Enumerable.Repeat((byte)0x53, 65536).ToArray();
                    for (var index = 0; index < 16; index++) second.Write(block);
                }
            }
            var events = new ConcurrentQueue<ArchiveWorkerEvent>();
            using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
            var id = Guid.NewGuid(); var cancelledDuringWrite = false;
            runner.OutputWriteCheckpointProbe = _ =>
            {
                if (cancelledDuringWrite || !File.Exists(Path.Combine(target.FullName, "second.bin"))) return;
                Check(File.ReadAllText(Path.Combine(target.FullName, "first.txt")) == "first-completed", "first file was not closed before second write");
                Check(new FileInfo(Path.Combine(target.FullName, "second.bin")).Length > 0, "second file has no written bytes");
                cancelledDuringWrite = true;
                Check(runner.Cancel(id), "cancel live original Run");
            };
            Check(runner.Start(new(id, "extract", archive, 0, new { max_entries = 100 }, OutputRoot: target.FullName)), "ZIP cancellation admission");
            var clock = Stopwatch.StartNew();
            while (!events.Any(e => e.Kind == "exited"))
            {
                Check(clock.ElapsedMilliseconds < 15000, "ZIP cancellation deadline");
                await Task.Delay(20);
            }
            Check(await runner.DrainAsync(15000), "cancelled Run drains before fixture disposal");
            var terminal = events.Single(e => e.Kind == "exited");
            Check(cancelledDuringWrite && terminal.Category == "cancelled_with_committed_output" &&
                terminal.CleanupComplete && terminal.FilesCommitted == 1 && terminal.Journal?.Count(f => f.Published == true) == 1,
                "cancel during second write retains first completed output and truthful journal: " + terminal.Category);
            Check(File.ReadAllText(Path.Combine(target.FullName, "first.txt")) == "first-completed" &&
                !File.Exists(Path.Combine(target.FullName, "second.bin")) &&
                Directory.GetFiles(target.FullName, "*", SearchOption.AllDirectories).Length == 1,
                "cancellation must remove current incomplete file only");
        }
        finally { if (copied) File.Delete(installed); owned.Delete(true); }
    }
    public static async Task ActualSevenzToHandleSession()
    {
        var source = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_CODEC_FIXTURES");
        if (string.IsNullOrEmpty(source) || !File.Exists(source) || string.IsNullOrEmpty(fixtures))
            throw new TestSkippedException("Explicit isolated-codecs worker and independent codec fixtures required.");
        var installed = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        if (File.Exists(installed)) throw new InvalidOperationException("Fixture cannot replace an existing worker.");
        var owned = Directory.CreateTempSubdirectory("archive-sevenz-chain-");
        var created = false;
        try
        {
            File.Copy(source, installed, false); created = true;
            var completedCases = 0;
            async Task<string> Verify(string name, string? password, object limits, string? expectedError = null, bool existing = false)
            {
                var target = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
                if (existing) File.WriteAllText(Path.Combine(target.FullName, "file.txt"), "user-original");
                if (expectedError is not null) File.WriteAllText(Path.Combine(target.FullName, "preserved-user.txt"), "unrelated-user-bytes");
                var events = new ConcurrentQueue<ArchiveWorkerEvent>();
                using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
                var path = Path.IsPathRooted(name) ? name : Path.Combine(fixtures, name);
                var request = new ArchiveWorkerRequest(Guid.NewGuid(), "extract", path, 0, limits, password, OutputRoot: target.FullName);
                if (!runner.Start(request)) throw new InvalidOperationException("7z extraction admission");
                var clock = Stopwatch.StartNew();
                while (!events.Any(e => e.Kind == "exited"))
                {
                    if (clock.ElapsedMilliseconds > 15000) throw new InvalidOperationException("7z extraction deadline");
                    await Task.Delay(20);
                }
                var terminal = events.Last();
                var failure = events.FirstOrDefault(e => e.Frame is { } f && f.GetProperty("op").GetString() == "failure")?.Frame;
                var code = failure?.GetProperty("error_code").GetString();
                if (!terminal.CleanupComplete || (expectedError is null ? terminal.Category != "success" : code != expectedError))
                    throw new InvalidOperationException("7z extraction result for " + Path.GetFileName(name) + ": expected=" + (expectedError ?? "success") + ", actual=" + terminal.Category + "/" + code + "; native=" + terminal.NativeErrorCode + "; cleanup=" + terminal.CleanupComplete);
                if (expectedError is not null && Directory.GetFiles(target.FullName, "*", SearchOption.AllDirectories).Length != (existing ? 2 : 1))
                    throw new InvalidOperationException("failed 7z extraction leaked output");
                if (existing && File.ReadAllText(Path.Combine(target.FullName, "file.txt")) != "user-original")
                    throw new InvalidOperationException("7z extraction clobbered existing target");
                if (expectedError is not null && File.ReadAllText(Path.Combine(target.FullName, "preserved-user.txt")) != "unrelated-user-bytes")
                    throw new InvalidOperationException("failed 7z extraction changed existing user file");
                if (events.Any(e => e.Frame is { } frame && frame.GetProperty("op").GetString() == "native_call"))
                    throw new InvalidOperationException("7z payload escaped private output pump");
                if (!await runner.DrainAsync()) throw new InvalidOperationException("7z extraction cleanup");
                completedCases++;
                Console.WriteLine($"ARCHIVE_7Z_CASE case={completedCases} fixture={Path.GetFileName(name)} expected={expectedError ?? "success"} actual={code ?? terminal.Category} cleanup={terminal.CleanupComplete} output_and_sentinel_checks=passed");
                return target.FullName;
            }
            var defaults = new { max_entries = 100 };
            var plain = await Verify("lzma.7z", null, defaults);
            if (!File.ReadAllBytes(Path.Combine(plain, "file.txt")).SequenceEqual(Encoding.UTF8.GetBytes("this is a file\n")))
                throw new InvalidOperationException("independent 7z bytes differ");
            foreach (var name in new[] { "solid.7z", "non-solid.7z" })
            {
                var target = await Verify(name, null, defaults);
                foreach (var (file, hash) in new[] {
                    ("test1.txt", "DAC1A4378FA936D14452116DD08D3DA46F9F5EB17A427A57ACE6ED7F2629919A"),
                    ("test2.txt", "91A5E97EE41057E078CE25E73F84A1D37F3C7C5C01B8D60A8D4E8C2B779C1392") })
                {
                    var actual = Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(Path.Combine(target, "test", file))));
                    if (actual != hash) throw new InvalidOperationException("independent solid/non-solid payload differs");
                }
                if (Directory.GetFiles(target, "*", SearchOption.AllDirectories).Length != 2)
                    throw new InvalidOperationException("solid/non-solid partial leaked");
            }
            var encrypted = await Verify("encrypted.7z", "sevenz-rust", defaults);
            AssertExactFiles(encrypted,
                ("encripted/7zFormat.txt", 2247, "09FF37C5CDB9D30953442827F21457A8263BAF7868471E5AE8317C743C84303B"),
                ("encripted/7ziplogo.png", 1417, "F8AC2201F43C4989EB95B2184EEE17A705A3107E43852859D5CE01E239D16ADA"));
            await Verify("encrypted.7z", null, defaults, "PasswordRequired");
            await Verify("encrypted.7z", "incorrect", defaults, "AuthenticationOrDataError");
            await Verify("lzma.7z", null, new { max_output_bytes = 1 }, "ResourceLimit");
            await Verify("lzma.7z", null, defaults, "OutputCollision", existing: true);
            var damaged = Path.Combine(owned.FullName, "damaged.7z");
            var bytes = File.ReadAllBytes(Path.Combine(fixtures, "lzma.7z"));
            bytes[40] ^= 0x80; File.WriteAllBytes(damaged, bytes);
            await Verify(damaged, null, defaults, "CorruptArchive");
            var encryptedDamage = Path.Combine(owned.FullName, "damaged-encrypted.7z");
            var encryptedBytes = File.ReadAllBytes(Path.Combine(fixtures, "encrypted.7z"));
            encryptedBytes[40] ^= 0x80; File.WriteAllBytes(encryptedDamage, encryptedBytes);
            await Verify(encryptedDamage, "sevenz-rust", defaults, "AuthenticationOrDataError");
            if (completedCases != 10) throw new InvalidOperationException("incomplete 7z preservation cases");
            Console.WriteLine("ARCHIVE_7Z_PRESERVATION cases=10 payload_oracles=passed collision=passed plain_corruption=passed encrypted_corruption=passed cleanup=passed");
        }
        finally { if (created) File.Delete(installed); owned.Delete(true); }
    }
    public static async Task ActualRarToHandleSession()
    {
        var source = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_CODEC_FIXTURES");
        if (string.IsNullOrEmpty(source) || !File.Exists(source) || string.IsNullOrEmpty(fixtures))
            throw new TestSkippedException("Explicit isolated-codecs worker and independent RAR fixtures required.");
        var installed = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        if (File.Exists(installed)) throw new InvalidOperationException("Fixture cannot replace an existing worker.");
        var owned = Directory.CreateTempSubdirectory("archive-rar-chain-");
        var created = false;
        try
        {
            File.Copy(source, installed, false); created = true;
            async Task<string> Verify(string name, string? password, object limits, string[]? expected = null, bool existing = false)
            {
                var target = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
                if (existing) File.WriteAllText(Path.Combine(target.FullName, "VERSION"), "user-original");
                if (expected is not null) File.WriteAllText(Path.Combine(target.FullName, "preserved-user.txt"), "unrelated-user-bytes");
                var events = new ConcurrentQueue<ArchiveWorkerEvent>();
                using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
                var path = Path.IsPathRooted(name) ? name : Path.Combine(fixtures, name);
                var request = new ArchiveWorkerRequest(Guid.NewGuid(), "extract", path, 0, limits, password, OutputRoot: target.FullName);
                if (!runner.Start(request)) throw new InvalidOperationException("RAR extraction admission");
                var clock = Stopwatch.StartNew();
                while (!events.Any(e => e.Kind == "exited"))
                {
                    if (clock.ElapsedMilliseconds > 15000) throw new InvalidOperationException("RAR extraction deadline");
                    await Task.Delay(20);
                }
                var failure = events.FirstOrDefault(e => e.Frame is { } f && f.GetProperty("op").GetString() == "failure")?.Frame;
                var code = failure?.GetProperty("error_code").GetString();
                if (!events.Last().CleanupComplete || (expected is null ? events.Last().Category != "success" : code is null || !expected.Contains(code)))
                    throw new InvalidOperationException("RAR extraction result: " + events.Last().Category + "/" + code);
                if (expected is not null && Directory.GetFiles(target.FullName, "*", SearchOption.AllDirectories).Length != (existing ? 2 : 1))
                    throw new InvalidOperationException("failed RAR extraction leaked output");
                if (existing && File.ReadAllText(Path.Combine(target.FullName, "VERSION")) != "user-original")
                    throw new InvalidOperationException("RAR extraction clobbered existing output");
                if (expected is not null && File.ReadAllText(Path.Combine(target.FullName, "preserved-user.txt")) != "unrelated-user-bytes")
                    throw new InvalidOperationException("failed RAR extraction changed existing user file");
                if (events.Any(e => e.Frame is { } frame && frame.GetProperty("op").GetString() == "native_call"))
                    throw new InvalidOperationException("RAR data escaped private output pump");
                if (!await runner.DrainAsync()) throw new InvalidOperationException("RAR extraction cleanup");
                return target.FullName;
            }
            var defaults = new { max_entries = 100 };
            var rar4 = await Verify("rar4.rar", null, defaults);
            if (!File.ReadAllBytes(Path.Combine(rar4, "VERSION")).SequenceEqual(Encoding.UTF8.GetBytes("unrar-0.4.0")))
                throw new InvalidOperationException("independent RAR4 bytes differ");
            var rar5 = await Verify("rar5.rar", null, defaults);
            var rar5Files = Directory.GetFiles(rar5, "*", SearchOption.AllDirectories);
            if (rar5Files.Length != 1 || !File.ReadAllBytes(rar5Files[0]).SequenceEqual(Encoding.UTF8.GetBytes("foobar\n")))
                throw new InvalidOperationException("independent RAR5 bytes differ");
            var encrypted4 = await Verify("rar4-encrypted.rar", "unrar", defaults);
            AssertExactFiles(encrypted4, (".gitignore", 18, "F9B1CA6AE27D1C18215265024629A8960C31379F206D9ED20F64E0B2DCF79805"));
            if (!File.ReadAllBytes(Path.Combine(encrypted4, ".gitignore")).SequenceEqual(Encoding.UTF8.GetBytes("target\nCargo.lock\n")))
                throw new InvalidOperationException("independent encrypted RAR4 literal differs");
            var encrypted5 = await Verify("rar5-header-encrypted.rar", "password", defaults);
            AssertExactFiles(encrypted5, (".gitignore", 18, "F9B1CA6AE27D1C18215265024629A8960C31379F206D9ED20F64E0B2DCF79805"));
            await Verify("rar4-encrypted.rar", null, defaults, ["PasswordRequired"]);
            await Verify("rar5-header-encrypted.rar", "incorrect", defaults, ["WrongPassword", "AuthenticationOrDataError"]);
            await Verify("rar4.rar", null, new { max_entry_bytes = 1 }, ["ResourceLimit"]);
            await Verify("rar4.rar", null, defaults, ["OutputCollision"], existing: true);
            var damaged = Path.Combine(owned.FullName, "damaged.rar");
            var bytes = File.ReadAllBytes(Path.Combine(fixtures, "rar4.rar"));
            bytes[^10] ^= 0x80; File.WriteAllBytes(damaged, bytes);
            await Verify(damaged, null, defaults, ["CorruptArchive"]);
            var encryptedDamage = Path.Combine(owned.FullName, "damaged-encrypted.rar");
            var encryptedBytes = File.ReadAllBytes(Path.Combine(fixtures, "rar4-encrypted.rar"));
            encryptedBytes[^10] ^= 0x80; File.WriteAllBytes(encryptedDamage, encryptedBytes);
            await Verify(encryptedDamage, "unrar", defaults, ["AuthenticationOrDataError"]);
            var encryptedDamage5 = Path.Combine(owned.FullName, "damaged-encrypted5.rar");
            var encryptedBytes5 = File.ReadAllBytes(Path.Combine(fixtures, "rar5-header-encrypted.rar"));
            encryptedBytes5[^30] ^= 0x80; File.WriteAllBytes(encryptedDamage5, encryptedBytes5);
            await Verify(encryptedDamage5, "password", defaults, ["AuthenticationOrDataError"]);
        }
        finally { if (created) File.Delete(installed); owned.Delete(true); }
    }
    // Independent 7-Zip 24.09 oracle, never produced by NxTools: fixtures/upstream/README.md.
    private static void AssertExactFiles(string root, params (string Name, long Size, string Hash)[] expected)
    {
        var files = Directory.GetFiles(root, "*", SearchOption.AllDirectories);
        if (files.Length != expected.Length) throw new InvalidOperationException("encrypted exact file count differs");
        foreach (var item in expected) {
            var path = Path.Combine(root, item.Name.Replace('/', Path.DirectorySeparatorChar));
            var bytes = File.ReadAllBytes(path);
            if (bytes.LongLength != item.Size || Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(bytes)) != item.Hash)
                throw new InvalidOperationException("independent encrypted payload hash differs");
        }
    }
}
