use super::*;
use std::{
    io::{self, Cursor},
    rc::Rc,
};
struct Input {
    data: Cursor<Vec<u8>>,
    eof: Rc<Cell<bool>>,
}
impl Read for Input {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let n = self.data.read(bytes)?;
        if n == 0 {
            self.eof.set(true);
        }
        Ok(n)
    }
}
struct Storage {
    eof: Rc<Cell<bool>>,
    fail_finish: bool,
    fail_flush: bool,
    hit: Rc<Cell<bool>>,
}
impl Write for Storage {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.fail_finish && self.eof.get() {
            self.hit.set(true);
            return Err(io::Error::other(ArchiveError::IoFailure));
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.fail_flush {
            self.hit.set(true);
            return Err(io::Error::other(ArchiveError::IoFailure));
        }
        Ok(())
    }
}
fn exercise(outer: Outer, finish: bool) {
    let eof = Rc::new(Cell::new(false));
    let hit = Rc::new(Cell::new(false));
    let mut input = Input {
        data: Cursor::new(vec![0; 2048]),
        eof: eof.clone(),
    };
    let storage = Storage {
        eof,
        fail_finish: finish,
        fail_flush: !finish,
        hit: hit.clone(),
    };
    let result = outer.encode_to(
        &mut input,
        storage,
        &ResourceLimits::default(),
        &OperationControl::default(),
    );
    assert!(
        hit.get(),
        "fault must reach actual edit encoder finish/final flush"
    );
    assert!(
        matches!(result, Err(ArchiveError::IoFailure)),
        "edit must not produce a publishable candidate"
    );
}
#[test]
fn tar_edit_gzip_encoder_finish_error_is_terminal() {
    exercise(
        Outer::Gzip {
            mtime: 0,
            os: 255,
            name: None,
            comment: None,
            level: 6,
        },
        true,
    );
}
#[test]
fn tar_edit_gzip_final_flush_error_is_terminal() {
    exercise(
        Outer::Gzip {
            mtime: 0,
            os: 255,
            name: None,
            comment: None,
            level: 6,
        },
        false,
    );
}
#[test]
fn tar_edit_xz_encoder_finish_error_is_terminal() {
    exercise(Outer::Xz { check: 4 }, true);
}
#[test]
fn tar_edit_xz_final_flush_error_is_terminal() {
    exercise(Outer::Xz { check: 4 }, false);
}

#[test]
fn tar_edit_outer_meaningful_properties_signature_and_independent_decode() {
    let root = tempfile::tempdir().expect("owned");
    let control = OperationControl::default();
    let limits = ResourceLimits::default();
    let script = r#"import sys,gzip,lzma,struct,zlib
mode,path=sys.argv[1:]
data=bytes(1024)
if mode=='gzip':
    fixed=bytearray(b'\x1f\x8b\x08\x18'+struct.pack('<I',1234567)+b'\x02\x03'); body=zlib.compress(data,9,wbits=-15);open(path,'wb').write(fixed+b'original.tar\0comment\0'+body+struct.pack('<II',zlib.crc32(data),len(data)))
else: open(path,'wb').write(lzma.compress(data,format=lzma.FORMAT_XZ,check=int(mode),preset=1))"#;
    for (mode, format) in [
        ("gzip", "TarGz"),
        ("0", "TarXz"),
        ("1", "TarXz"),
        ("4", "TarXz"),
        ("10", "TarXz"),
    ] {
        let path = root.path().join("source");
        assert!(
            std::process::Command::new("python")
                .args(["-c", script, mode])
                .arg(&path)
                .status()
                .expect("independent outer seed")
                .success()
        );
        let (outer, mut plain) = Outer::open(
            File::open(&path).expect("source"),
            format,
            &limits,
            &control,
        )
        .expect("qualified");
        if mode == "gzip" {
            assert!(
                matches!(&outer,Outer::Gzip{mtime:1234567,os:3,name:Some(n),comment:Some(c),level:9} if n==b"original.tar"&&c==b"comment")
            );
        }
        let encoded = outer
            .encode_to(&mut plain, Vec::new(), &limits, &control)
            .expect("encode");
        let candidate = root.path().join("candidate");
        std::fs::write(&candidate, encoded).expect("candidate");
        let (actual, _) = Outer::open(
            File::open(&candidate).expect("candidate"),
            format,
            &limits,
            &control,
        )
        .expect("outer full readback");
        assert!(actual == outer, "meaningful outer properties");
        assert!(std::process::Command::new("python").args(["-c","import sys,gzip,lzma; b=open(sys.argv[1],'rb').read(); assert (gzip.decompress(b) if sys.argv[2]=='TarGz' else lzma.decompress(b))==bytes(1024)"]).arg(&candidate).arg(format).status().expect("independent full decode").success());
        let wrong = if format == "TarGz" { "TarXz" } else { "TarGz" };
        assert!(matches!(
            Outer::open(File::open(&path).expect("source"), wrong, &limits, &control),
            Err(ArchiveError::UnsupportedOperation)
        ));
    }
}
