use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use fuse_mt::*;
use libc;
use log::*;
use time::Timespec;

type Blob = Vec<u8>;
type LibCError = libc::c_int;

enum Entry {
    File { mtime_sec: i64, blob: Blob },
    Dir { mtime_sec: i64 },
}

#[derive(Default)]
struct FsImpl {
    uid: u32,
    gid: u32,
    entries: HashMap<PathBuf, Entry>,
}

impl FsImpl {
    fn new(uid: u32, gid: u32) -> Self {
        let mut entries = HashMap::with_capacity(128);
        entries.insert(
            PathBuf::from("/"),
            Entry::Dir {
                mtime_sec: Self::now_utc(),
            },
        );
        FsImpl { uid, gid, entries }
    }

    fn now_utc() -> i64 {
        time::now_utc().to_timespec().sec
    }

    fn auth(&self, req: RequestInfo) -> ResultEmpty {
        if req.gid != self.gid || req.uid != self.uid {
            info!("{}: Unauthorized for {}:{}", req.unique, req.uid, req.gid);
            Err(libc::EACCES)
        } else {
            Ok(())
        }
    }

    fn blob_mut(&mut self, path: &Path) -> Result<&mut Blob, LibCError> {
        match self.entries.get_mut(path).ok_or(libc::ENOENT)? {
            Entry::Dir { .. } => Err(libc::EISDIR),
            Entry::File { blob, .. } => Ok(blob),
        }
    }

    fn truncate(&mut self, path: &Path, size: u64) -> ResultEmpty {
        let blob = self.blob_mut(path)?;
        blob.resize_with(size as usize, Default::default);
        Ok(())
    }

    fn remove(&mut self, parent: &Path, name: &OsStr, need_dir: bool) -> ResultEmpty {
        let entry = parent.join(name);
        if let Some(is_dir) = self.is_dir(&entry) {
            if is_dir != need_dir {
                Err(if need_dir {
                    libc::ENOTDIR
                } else {
                    libc::EISDIR
                })
            } else {
                self.entries.remove(&entry);
                Ok(())
            }
        } else {
            Err(libc::ENOENT)
        }
    }

    fn attr(&self, size: u64, mtime_sec: i64, is_dir: bool) -> FileAttr {
        let blocks = size; // trying to use blksize=1
        let mtime = Timespec::new(mtime_sec, 0);
        let kind = if is_dir {
            fuse::FileType::Directory
        } else {
            fuse::FileType::RegularFile
        };
        FileAttr {
            size,
            blocks,
            atime: mtime,
            mtime,
            ctime: mtime,
            crtime: mtime,
            kind,
            perm: 0o700,
            nlink: 1,
            uid: self.uid,
            gid: self.gid,
            rdev: 0,
            flags: 0, /* macOS only; see chflags(2) */
        }
    }

    fn dir_entry(&self, mtime_sec: i64) -> (Timespec, FileAttr) {
        (Timespec::new(mtime_sec, 0), self.attr(0, mtime_sec, true))
    }

    fn file_entry(&self, mtime_sec: i64, size: usize) -> (Timespec, FileAttr) {
        (
            Timespec::new(mtime_sec, 0),
            self.attr(size as u64, mtime_sec, false),
        )
    }

    fn mkdir(&mut self, parent: &Path, name: &OsStr) -> ResultEntry {
        let entry = parent.join(name);
        if let Some(existing) = self.entries.get(&entry) {
            match existing {
                Entry::Dir { mtime_sec } => Ok(self.dir_entry(*mtime_sec)),
                Entry::File { .. } => Err(libc::ENOTDIR),
            }
        } else {
            let mtime_sec = Self::now_utc();
            self.entries.insert(entry, Entry::Dir { mtime_sec });
            Ok(self.dir_entry(mtime_sec))
        }
    }

    fn is_dir(&self, path: &Path) -> Option<bool> {
        if let Some(existing) = self.entries.get(path) {
            match existing {
                Entry::Dir { .. } => Some(true),
                Entry::File { .. } => Some(false),
            }
        } else {
            None
        }
    }

    fn getattr(&self, path: &Path) -> ResultEntry {
        if let Some(existing) = self.entries.get(path) {
            match existing {
                Entry::Dir { mtime_sec } => Ok(self.dir_entry(*mtime_sec)),
                Entry::File { mtime_sec, blob } => Ok(self.file_entry(*mtime_sec, blob.len())),
            }
        } else {
            Err(libc::ENOENT)
        }
    }

    fn readdir(&self, path: &Path) -> ResultReaddir {
        let is_dir_opt = self.is_dir(path);
        if is_dir_opt.is_none() {
            return Err(libc::ENOENT);
        } else if is_dir_opt == Some(false) {
            return Err(libc::ENOTDIR);
        }

        // TODO Linear search through all files is fine for now
        let mut dir = Vec::with_capacity(64);
        for entry in &self.entries {
            let parent_opt = entry.0.parent();
            if parent_opt.is_some() && parent_opt.unwrap() == path {
                let kind = match entry.1 {
                    Entry::Dir { .. } => fuse::FileType::Directory,
                    Entry::File { .. } => fuse::FileType::RegularFile,
                };
                let name = entry.0.file_name().unwrap().to_owned();
                dir.push(DirectoryEntry { name, kind });
            }
        }
        Ok(dir)
    }
}

#[derive(Default)]
pub struct ForgetfulFS {
    inner: RwLock<FsImpl>,
}

impl ForgetfulFS {
    pub fn new(uid: u32, gid: u32) -> Self {
        let inner = RwLock::new(FsImpl::new(uid, gid));
        Self { inner }
    }

    fn rlock<T, F>(&self, req: RequestInfo, f: F) -> Result<T, LibCError>
    where
        F: FnOnce(&RwLockReadGuard<'_, FsImpl>) -> Result<T, LibCError>,
    {
        trace!("{}: Trying to acquire read lock", req.unique);
        match self.inner.read() {
            Err(_e) => {
                debug!("{}: Could not acquire read lock", req.unique);
                Err(libc::EAGAIN)
            }
            Ok(this) => {
                this.auth(req)?;
                let res = f(&this);
                trace!("{}: Releasing read lock", req.unique);
                res
            }
        }
    }

    fn wlock<T, F>(&self, req: RequestInfo, f: F) -> Result<T, LibCError>
    where
        F: FnOnce(&mut RwLockWriteGuard<'_, FsImpl>) -> Result<T, LibCError>,
    {
        trace!("{}: Trying to acquire write lock", req.unique);
        match self.inner.write() {
            Err(_e) => {
                debug!("{}: Could not acquire write lock", req.unique);
                Err(libc::EAGAIN)
            }
            Ok(mut this) => {
                this.auth(req)?;
                let res = f(&mut this);
                trace!("{}: Releasing write lock", req.unique);
                res
            }
        }
    }
}

impl FilesystemMT for ForgetfulFS {
    fn init(&self, req: RequestInfo) -> ResultEmpty {
        info!(
            "{}: init {}:{} from PID {}",
            req.unique, req.uid, req.gid, req.pid
        );
        Ok(())
    }

    fn truncate(&self, req: RequestInfo, path: &Path, _fh: Option<u64>, size: u64) -> ResultEmpty {
        info!(
            "{}: truncate {}, {}",
            req.unique,
            path.to_string_lossy(),
            size
        );
        self.wlock(req, |this| this.truncate(path, size))
    }

    fn unlink(&self, req: RequestInfo, parent: &Path, name: &OsStr) -> ResultEmpty {
        info!(
            "{}: unlink {}, {}",
            req.unique,
            parent.to_string_lossy(),
            name.to_string_lossy()
        );
        self.wlock(req, |this| this.remove(parent, name, false))
    }

    fn rmdir(&self, req: RequestInfo, parent: &Path, name: &OsStr) -> ResultEmpty {
        info!(
            "{}: rmdir {}, {}",
            req.unique,
            parent.to_string_lossy(),
            name.to_string_lossy()
        );
        self.wlock(req, |this| this.remove(parent, name, true))
    }

    fn mkdir(&self, req: RequestInfo, parent: &Path, name: &OsStr, _mode: u32) -> ResultEntry {
        info!(
            "{}: mkdir {}, {}",
            req.unique,
            parent.to_string_lossy(),
            name.to_string_lossy()
        );
        self.wlock(req, |this| this.mkdir(parent, name))
    }

    fn opendir(&self, req: RequestInfo, path: &Path, _flags: u32) -> ResultOpen {
        info!("{}: opendir {}", req.unique, path.to_string_lossy());
        self.rlock(req, |this| {
            let is_dir_opt = this.is_dir(path);
            if is_dir_opt.is_none() {
                Err(libc::ENOENT)
            } else if is_dir_opt == Some(false) {
                Err(libc::ENOTDIR)
            } else {
                Ok((0, 0))
            }
        })
    }

    fn readdir(&self, req: RequestInfo, path: &Path, _fh: u64) -> ResultReaddir {
        info!("{}: readdir {}", req.unique, path.to_string_lossy());
        self.rlock(req, |this| this.readdir(path))
    }

    fn getattr(&self, req: RequestInfo, path: &Path, _fh: Option<u64>) -> ResultEntry {
        info!("{}: getattr {}", req.unique, path.to_string_lossy());
        self.rlock(req, |this| this.getattr(path))
    }

    fn statfs(&self, req: RequestInfo, path: &Path) -> ResultStatfs {
        info!("{}: statfs {}", req.unique, path.to_string_lossy());
        let statfs = Statfs {
            blocks: 0u64,
            bfree: 0u64,
            bavail: 0u64,
            files: 0u64,
            ffree: 0u64,
            bsize: std::u32::MAX,
            namelen: std::u32::MAX,
            frsize: 1u32,
        };
        self.rlock(req, |_this| Ok(statfs))
    }
}
