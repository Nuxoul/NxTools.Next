mod input;
mod names;
mod stream;
mod zip;
use input::Parts;
use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum Kind {
    Unsupported,
    Corrupt,
    Limit,
    UnsafeName,
    Collision,
    Io,
    TargetExists,
    Usage,
}
#[derive(Debug)]
pub struct Error {
    pub kind: Kind,
    pub native: Option<i32>,
    pub completed: Summary,
    pub cleanup_failed: bool,
    pub cleanup_native: Option<i32>,
}
impl Error {
    fn new(kind: Kind) -> Self {
        Self {
            kind,
            native: None,
            completed: Summary::default(),
            cleanup_failed: false,
            cleanup_native: None,
        }
    }
    fn corrupt() -> Self {
        Self::new(Kind::Corrupt)
    }
}
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        let mut result = Self::new(if error.kind() == io::ErrorKind::UnexpectedEof {
            Kind::Corrupt
        } else {
            Kind::Io
        });
        result.native = error.raw_os_error();
        result
    }
}
pub type Result<T> = std::result::Result<T, Error>;
fn require(condition: bool) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::corrupt())
    }
}
fn add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).ok_or_else(Error::corrupt)
}
#[derive(Clone, Copy)]
pub struct Limits {
    pub metadata: u64,
    pub entries: u64,
    pub name: usize,
    pub disks: u32,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            metadata: 64 * 1024 * 1024,
            entries: 65536,
            name: 4096,
            disks: 1024,
        }
    }
}
pub struct Entry {
    pub name: String,
    pub size: u64,
    pub method: u16,
    pub ordinal: usize,
    pub disk: u32,
    pub offset: u64,
    pub directory: bool,
    packed: u64,
    crc: u32,
    flags: u16,
    version: u16,
    time: u32,
    wide: bool,
    data: u64,
    record_end: u64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub files: u64,
    pub directories: u64,
    pub bytes: u64,
}
pub struct Archive {
    pub entries: Vec<Entry>,
    parts: Parts,
}
impl Archive {
    pub fn open(path: &Path, limits: Limits) -> Result<Self> {
        let mut parts = Parts::open(path, limits)?;
        let entries = zip::catalog(&mut parts, limits)?;
        entries
            .iter()
            .try_fold(0u64, |total, e| add(total, e.size))?;
        Ok(Self { entries, parts })
    }
    pub fn test(&mut self) -> Result<Summary> {
        let mut summary = Summary::default();
        for entry in &self.entries {
            stream::decode(&mut self.parts, entry, &mut io::sink()).map_err(|mut e| {
                e.completed = summary;
                e
            })?;
            summary.record(entry)?;
        }
        Ok(summary)
    }
    pub fn extract(&mut self, root: &Path) -> Result<Summary> {
        if !root.is_absolute() {
            return Err(Error::new(Kind::Usage));
        }
        fs::create_dir(root).map_err(|e| {
            if e.kind() == io::ErrorKind::AlreadyExists {
                let mut error = Error::new(Kind::TargetExists);
                error.native = e.raw_os_error();
                error
            } else {
                e.into()
            }
        })?;
        let catalog: BTreeSet<_> = self.entries.iter().map(|e| names::key(&e.name)).collect();
        let mut summary = Summary::default();
        for entry in &self.entries {
            let result = if entry.directory {
                stream::decode(&mut self.parts, entry, &mut io::sink()).and_then(|()| {
                    fs::create_dir_all(root.join(&entry.name))?;
                    Ok(())
                })
            } else {
                extract_file(&mut self.parts, entry, root, &catalog)
            };
            result.map_err(|mut e| {
                e.completed = summary;
                e
            })?;
            summary.record(entry)?;
        }
        Ok(summary)
    }
}
impl Summary {
    fn record(&mut self, entry: &Entry) -> Result<()> {
        if entry.directory {
            self.directories += 1;
        } else {
            self.files += 1;
        }
        self.bytes = add(self.bytes, entry.size)?;
        Ok(())
    }
}
fn extract_file(
    parts: &mut Parts,
    entry: &Entry,
    root: &Path,
    catalog: &BTreeSet<String>,
) -> Result<()> {
    let parent = entry.name.rsplit_once('/').map_or("", |(p, _)| p);
    let parent_path = root.join(parent);
    fs::create_dir_all(&parent_path)?;
    let mut candidate = None;
    for attempt in 0..=catalog.len() {
        let leaf = format!(".nxtools-sfx-{}-{attempt}.partial", entry.ordinal);
        let relative = if parent.is_empty() {
            leaf
        } else {
            format!("{parent}/{leaf}")
        };
        let key = names::key(&relative);
        let prefix = format!("{key}/");
        if catalog
            .range(key.clone()..)
            .next()
            .is_none_or(|next| next != &key && !next.starts_with(&prefix))
        {
            candidate = Some(root.join(relative));
            break;
        }
    }
    let candidate = candidate.ok_or_else(|| Error::new(Kind::Collision))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&candidate)?;
    let result = stream::decode(parts, entry, &mut file).and_then(|()| {
        file.flush()?;
        Ok(())
    });
    drop(file);
    let result = result.and_then(|()| {
        tempfile::TempPath::try_from_path(candidate.clone())?
            .persist_noclobber(root.join(&entry.name))
            .map_err(|mut failure| {
                failure.path.disable_cleanup(true);
                Error::from(failure.error)
            })
    });
    if let Err(mut error) = result {
        if let Err(cleanup) = fs::remove_file(&candidate) {
            error.cleanup_failed = true;
            error.cleanup_native = cleanup.raw_os_error();
        }
        return Err(error);
    }
    Ok(())
}
#[cfg(test)]
fn validate_names(names: &[(String, bool)], limit: usize) -> Result<()> {
    names::validate(names.iter().map(|(n, d)| (n.as_str(), *d)), limit)
}
const NOTICES: &[(&str, &str)] = &[
    (
        "NxTools MIT",
        include_str!("../../notices/zip-sfx/project-MIT.txt"),
    ),
    (
        "NxTools Apache notice",
        include_str!("../../notices/zip-sfx/project-APACHE.txt"),
    ),
    (
        "Apache License 2.0",
        include_str!("../../notices/zip-sfx/Apache-2.0.txt"),
    ),
    (
        "flate2 1.1.9 MIT",
        include_str!("../../notices/zip-sfx/flate2-1.1.9-MIT.txt"),
    ),
    (
        "crc32fast 1.5.0 MIT",
        include_str!("../../notices/zip-sfx/crc32fast-1.5.0-MIT.txt"),
    ),
    (
        "cfg-if 1.0.4 MIT",
        include_str!("../../notices/zip-sfx/cfg-if-1.0.4-MIT.txt"),
    ),
    (
        "zlib-rs 0.6.6 zlib",
        include_str!("../../notices/zip-sfx/zlib-rs-0.6.6.txt"),
    ),
    (
        "tempfile 3.27.0 MIT",
        include_str!("../../notices/zip-sfx/tempfile-3.27.0-MIT.txt"),
    ),
    (
        "fastrand 2.4.1 MIT",
        include_str!("../../notices/zip-sfx/fastrand-2.4.1-MIT.txt"),
    ),
    (
        "once_cell 1.21.4 MIT",
        include_str!("../../notices/zip-sfx/once_cell-1.21.4-MIT.txt"),
    ),
    (
        "getrandom 0.4.3 MIT",
        include_str!("../../notices/zip-sfx/getrandom-0.4.3-MIT.txt"),
    ),
    (
        "windows-sys 0.61.2 MIT",
        include_str!("../../notices/zip-sfx/windows-sys-0.61.2-MIT.txt"),
    ),
    (
        "windows-link 0.2.1 MIT",
        include_str!("../../notices/zip-sfx/windows-link-0.2.1-MIT.txt"),
    ),
    (
        "Rust standard library",
        include_str!("../../notices/zip-sfx/rust-1.96.0-std.txt"),
    ),
];
fn line() -> Result<String> {
    let mut text = String::new();
    let count = io::stdin().lock().take(32770).read_line(&mut text)?;
    if count == 0 || count > 32768 || !text.ends_with('\n') {
        return Err(Error::new(Kind::Usage));
    }
    Ok(text.trim_end_matches(['\r', '\n']).to_owned())
}
pub fn run() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).take(4).collect();
    if args.len() == 1 && args[0] == "--licenses" {
        let mut output = io::stdout().lock();
        for (name, text) in NOTICES {
            writeln!(output, "=== {name} ===\n{text}")?;
        }
        return Ok(());
    }
    let test = args.len() == 1 && args[0] == "--test";
    let prompt = args.is_empty();
    if !(test || prompt || (args.len() == 2 && args[0] == "--output")) {
        return Err(Error::new(Kind::Usage));
    }
    println!("stage=accepted");
    let own = std::env::current_exe()?;
    let mut archive = Archive::open(&own, Limits::default())?;
    println!(
        "stage=catalog entries={} disks={}",
        archive.entries.len(),
        archive.parts.count()
    );
    let summary = if test {
        println!("stage=testing");
        archive.test()?
    } else {
        let root = if prompt {
            let stem = own.file_stem().ok_or_else(|| Error::new(Kind::Usage))?;
            let default = own.with_file_name(stem);
            print!("Destination [{}]: ", default.display());
            io::stdout().flush()?;
            let requested = line()?;
            let root = if requested.is_empty() {
                default
            } else {
                PathBuf::from(requested)
            };
            println!("Press Enter to confirm extraction; any other input cancels.");
            if !line()?.is_empty() {
                return Err(Error::new(Kind::Usage));
            }
            root
        } else {
            PathBuf::from(&args[1])
        };
        println!("stage=extracting");
        archive.extract(&root)?
    };
    println!(
        "stage=succeeded files={} directories={} bytes={}",
        summary.files, summary.directories, summary.bytes
    );
    Ok(())
}
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_publication;
