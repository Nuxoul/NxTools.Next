using System.Runtime.InteropServices;
using System.Security.Cryptography;
using Microsoft.Win32.SafeHandles;

namespace NxTools.PlatformHost.Windows.Extensions;

// Generic Credential Manager blob native cap is 5*512 bytes, below the broker's 32KiB
// protocol ceiling. Fail explicitly rather than fragmenting secrets into non-atomic writes.
internal sealed class CredentialWindowsStore : ICredentialStore
{
    internal const int NativeBlobLimit = 2560;
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct Credential
    {
        internal uint Flags, Type;
        internal IntPtr Target, Comment;
        internal System.Runtime.InteropServices.ComTypes.FILETIME LastWritten;
        internal uint BlobSize;
        internal IntPtr Blob;
        internal uint Persist, AttributeCount;
        internal IntPtr Attributes, Alias, User;
    }
    // CredReadW allocation owns credential, strings and blob; free exactly once with CredFree.
    private sealed class CredentialReadHandle : SafeHandleZeroOrMinusOneIsInvalid
    {
        private CredentialReadHandle() : base(true) { }
        protected override bool ReleaseHandle() { Free(handle); return true; }
    }
    [DllImport("advapi32.dll", EntryPoint = "CredReadW", ExactSpelling = true, CharSet = CharSet.Unicode, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)] private static extern bool ReadNative(string target, uint type, uint flags, out CredentialReadHandle credential);
    [DllImport("advapi32.dll", EntryPoint = "CredWriteW", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)] private static extern bool WriteNative(ref Credential credential, uint flags);
    [DllImport("advapi32.dll", EntryPoint = "CredDeleteW", ExactSpelling = true, CharSet = CharSet.Unicode, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)] private static extern bool DeleteNative(string target, uint type, uint flags);
    [DllImport("advapi32.dll", EntryPoint = "CredFree", ExactSpelling = true)]
    private static extern void Free(IntPtr buffer);

    public string? Read(string target)
    {
        ExtensionNative.Ordinary(ExtensionNative.CurrentProcess());
        if (!ReadNative(target, 1, 0, out var handle))
        {
            var code = Marshal.GetLastWin32Error(); handle?.Dispose();
            if (code == 1168) return null;
            throw new ExtensionFailure("credential_read_failed", code);
        }
        using (handle)
        {
            var credential = Marshal.PtrToStructure<Credential>(handle.DangerousGetHandle());
            if (credential.Type != 1 || credential.BlobSize > NativeBlobLimit ||
                (credential.BlobSize != 0 && credential.Blob == IntPtr.Zero)) throw new ExtensionFailure("credential_native_blob_invalid");
            var bytes = new byte[credential.BlobSize];
            try
            {
                if (bytes.Length != 0) Marshal.Copy(credential.Blob, bytes, 0, bytes.Length);
                try { return ExtensionProcessProtocol.Utf8.GetString(bytes); }
                catch (System.Text.DecoderFallbackException) { throw new ExtensionFailure("credential_invalid_utf8"); }
            }
            finally { CryptographicOperations.ZeroMemory(bytes); Zero(credential.Blob, (int)credential.BlobSize); }
        }
    }
    public void Write(string target, string value)
    {
        ExtensionNative.Ordinary(ExtensionNative.CurrentProcess());
        CredentialAdapter.ValidateValue(value);
        var bytes = ExtensionProcessProtocol.Utf8.GetBytes(value);
        if (bytes.Length > NativeBlobLimit) { CryptographicOperations.ZeroMemory(bytes); throw new ExtensionFailure("credential_native_value_limit"); }
        var blob = IntPtr.Zero; var name = IntPtr.Zero; var user = IntPtr.Zero;
        try
        {
            blob = Marshal.AllocHGlobal(Math.Max(1, bytes.Length));
            name = Marshal.StringToHGlobalUni(target); user = Marshal.StringToHGlobalUni("NxTools.Next");
            if (bytes.Length != 0) Marshal.Copy(bytes, 0, blob, bytes.Length);
            var credential = new Credential { Type = 1, Target = name, User = user, Blob = blob, BlobSize = (uint)bytes.Length, Persist = 2 };
            if (!WriteNative(ref credential, 0)) throw new ExtensionFailure("credential_write_failed", Marshal.GetLastWin32Error());
        }
        finally
        {
            Zero(blob, bytes.Length); CryptographicOperations.ZeroMemory(bytes);
            Marshal.FreeHGlobal(blob); Marshal.FreeHGlobal(name); Marshal.FreeHGlobal(user);
        }
    }
    public void Delete(string target)
    {
        ExtensionNative.Ordinary(ExtensionNative.CurrentProcess());
        if (!DeleteNative(target, 1, 0))
        {
            var code = Marshal.GetLastWin32Error();
            if (code != 1168) throw new ExtensionFailure("credential_delete_failed", code);
        }
    }
    private static void Zero(IntPtr buffer, int length)
    { if (buffer != IntPtr.Zero) for (var index = 0; index < length; index++) Marshal.WriteByte(buffer, index, 0); }
}
