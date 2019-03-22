use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use fuse_mt::*;
use libc;

type Blob = Vec<u8>;
type LibCError = libc::c_int;

enum Entry {
    File(Blob),
    Dir,
}

#[derive(Default)]
struct FsImpl {
    uid: u32,
    gid: u32,
    entries: HashMap<PathBuf, Entry>,
}

impl FsImpl {
    fn new() -> Self {
        let uid = Default::default();
        let gid = Default::default();
        let entries = Default::default();
        FsImpl { uid, gid, entries }
    }

    fn auth(&self, req: RequestInfo) -> ResultEmpty {
        if req.gid != self.gid || req.uid != self.uid {
            Err(libc::EACCES)
        } else {
            Ok(())
        }
    }

    fn blob_mut(&mut self, path: &Path) -> Result<&mut Blob, LibCError> {
        match self.entries.get_mut(path).ok_or(libc::ENOENT)? {
            Entry::Dir => Err(libc::EISDIR),
            Entry::File(blob) => Ok(blob),
        }
    }

    fn truncate(&mut self, path: &Path, size: u64) -> ResultEmpty {
        let blob = self.blob_mut(path)?;
        blob.resize_with(size as usize, Default::default);
        Ok(())
    }

    fn unlink(&mut self, parent: &Path, name: &OsStr) -> ResultEmpty {
        let entry = parent.join(name);
        self.entries.remove(&entry);
        Ok(())
    }
}

#[derive(Default)]
struct ForgetfulFS {
    inner: RwLock<FsImpl>,
}

impl ForgetfulFS {
    fn new() -> Self {
        let inner = RwLock::new(FsImpl::new());
        Self { inner }
    }

    fn rlock<T, F>(&self, req: RequestInfo, f: F) -> Result<T, LibCError>
    where
        F: FnOnce(&RwLockReadGuard<'_, FsImpl>) -> Result<T, LibCError>,
    {
        match self.inner.read() {
            Err(_e) => Err(libc::EAGAIN),
            Ok(this) => {
                this.auth(req)?;
                f(&this)
            }
        }
    }

    fn wlock<T, F>(&self, req: RequestInfo, f: F) -> Result<T, LibCError>
    where
        F: FnOnce(&mut RwLockWriteGuard<'_, FsImpl>) -> Result<T, LibCError>,
    {
        match self.inner.write() {
            Err(_e) => Err(libc::EAGAIN),
            Ok(mut this) => {
                this.auth(req)?;
                f(&mut this)
            }
        }
    }
}

impl fuse::Filesystem for ForgetfulFS {}

impl FilesystemMT for ForgetfulFS {
    fn init(&self, req: RequestInfo) -> ResultEmpty {
        self.wlock(req, |this| {
            this.uid = req.uid;
            this.gid = req.gid;
            Ok(())
        })
    }

    fn truncate(&self, req: RequestInfo, path: &Path, _fh: Option<u64>, size: u64) -> ResultEmpty {
        self.wlock(req, |this| this.truncate(path, size))
    }

    fn unlink(&self, req: RequestInfo, parent: &Path, name: &OsStr) -> ResultEmpty {
        self.wlock(req, |this| this.unlink(parent, name))
    }
}

fn main() -> std::io::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }

    let mount = &args[1];
    println!("{}", mount);
    let fs = ForgetfulFS::new();
    let options = [
        OsStr::new("-o"),
        OsStr::new("auto_unmount,default_permissions"),
    ];
    fuse_mt::mount(fs, mount, &options[..])
}
