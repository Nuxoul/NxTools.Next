# Windows long paths — reference-first source ledger

Source-only reinspection2026-10-20 before test implementation. No reference executable,
Windows >260/UNC runtime probe or Bandizip reverse engineering. The Next real-chain contract
is [windows-long-path-contract.md](../windows-long-path-contract.md). Ordinary Path/File APIs
and lexical guards are not runtime long-path/no-reparse/publication safety evidence.

## Legacy NxTools

Root `D:\Code\NxTools`. No general archive browser/extractor lifecycle was identified by scoped
tracked source search; this is not an exhaustive generated/untracked search.
`Services/BrowserExtensionPackageService.cs:214-254` enumerates files, Path.GetRelativePath,
backslash-to-slash ZIP names and streaming writes.152-173 deletes an existing destination before
moving the temporary;134-149 falls back to numbered output. Failure after deletion can lose the
old destination. `Services/FfmpegInstallerService.cs:87-109,131-140` calls
ZipFile.ExtractToDirectory(overwrite:true), then direct File.Copy(overwrite:true); synchronous
extract/copy do not observe HTTP cancellation. Neither is a safe general archive transaction.
NxTools.csproj targets net8 Windows; app.manifest1-18 has no explicit longPathAware element.
No archive-I/O verbatim-prefix conversion was identified. This is not runtime failure proof.

## Archiver

Root `D:\Code\Archiver-main`. GUI main.rs403-435/485-508 routes list/extract/create through its
single background worker. `core/src/lib.rs:494-577` lists ordinary File::open/codec paths;
583-689 extracts through enclosed_name/safe_join then direct File::create or backend unpack.
The safe_join202-213 guard is lexical, not existing-reparse/parent-swap protection.859-872
collects relative creation names with lossy conversion;926-1073 writes the final archive
directly and removes it on failure/cancel. ZIP edit765-825 uses predictable rewrite.tmp then
rename without expected old identity. gui/src/shell.rs506-512 strips the registered executable's
verbatim prefix only, not archive-I/O paths. No inspected archive-I/O >260Windows regression.

## SupaZip

Root `D:\Code\supazip-master\supazip`. CLI src/main.rs296-343/404-418 opens ordinary File inputs;
346-401 buffers complete created output then NamedTempFile/sync_all/persist (no old-target ID).
GUI main.rs612-637 creates the final target before encoding. Core formats/zip.rs120-135/214-226
buffers input;320/419 directly creates extracted targets;489-520 stores supplied PathBuf display
names,735-804 explicitly acknowledges absolute creation names. sevenz.rs413/tar.rs237 likewise
writes final extraction paths. No danger classification in public ArchiveEntry or native
protected publication is established; unsafe member skipping is not copied to Next. Cancellation
can leave partial direct output; CLI cancelled flag is not actually flipped in inspected496-522.
No inspected source proves runtime >260 Windows support or safe UNC behavior.

## Retained prior source-navigation facts

The condensed main [archive reference index](../../../../docs/reference-code/archive.md) links:
- [product read constructor/IPC/Job/helper and repeated legacy/Archiver chains](../product-read-contract.md);
- [resource boundaries](resource-boundaries.md);
- [finite100k budgets/bootstrap/original cleanup](../product-100k-contract.md).
No old general archive lifecycle was found; fixture helper copying is not deployment acceptance.
Those preexisting contracts/evidence are retained unchanged. Next's stronger source/native output
contracts and approved rechecked atomic-path TOCTOU remain authoritative, not reference direct writes.
