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

    fn utimens(&mut self, path: &Path, mtime: Option<Timespec>) -> ResultEmpty {
        let entry: &mut Entry = self.entries.get_mut(path).ok_or(libc::ENOENT)?;
        let x: &mut i64 = match entry {
            Entry::Dir { mtime_sec } => mtime_sec,
            Entry::File { mtime_sec, .. } => mtime_sec,
        };
        if mtime.is_some() {
            *x = mtime.unwrap().sec;
        }
        Ok(())
    }

    fn unlink(&mut self, parent: &Path, name: &OsStr) -> ResultEmpty {
        let file_path = parent.join(name);
        self.check_entry(&file_path, false)?;
        self.entries.remove(&file_path);
        Ok(())
    }

    fn rmdir(&mut self, parent: &Path, name: &OsStr) -> ResultEmpty {
        let dir_path = parent.join(name);
        self.check_entry(&dir_path, true)?;
        for entry in &self.entries {
            if entry.0.parent() == Some(&dir_path) {
                return Err(libc::ENOTEMPTY);
            }
        }
        self.entries.remove(&dir_path);
        Ok(())
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
        let dir_name = parent.join(name);
        self.check_entry(parent, true)?;
        if let Some(existing) = self.entries.get(&dir_name) {
            match existing {
                Entry::Dir { mtime_sec } => Ok(self.dir_entry(*mtime_sec)),
                Entry::File { .. } => Err(libc::ENOTDIR),
            }
        } else {
            let mtime_sec = Self::now_utc();
            self.entries.insert(dir_name, Entry::Dir { mtime_sec });
            Ok(self.dir_entry(mtime_sec))
        }
    }

    fn create(&mut self, parent: &Path, name: &OsStr) -> ResultCreate {
        let file_name = parent.join(name);
        self.check_entry(parent, true)?;
        if let Some(_existing) = self.entries.get(&file_name) {
            Err(libc::EEXIST)
        } else {
            let mtime_sec = Self::now_utc();
            let blob = Vec::with_capacity(1024);
            self.entries
                .insert(file_name, Entry::File { mtime_sec, blob });
            let res = CreatedEntry {
                ttl: Timespec::new(std::i64::MAX, 0),
                attr: self.attr(0, mtime_sec, false),
                fh: 0,
                flags: 0,
            };
            Ok(res)
        }
    }

    fn check_entry(&self, path: &Path, is_dir: bool) -> ResultEmpty {
        if let Some(existing) = self.entries.get(path) {
            match existing {
                Entry::Dir { .. } => {
                    if is_dir {
                        Ok(())
                    } else {
                        Err(libc::EISDIR)
                    }
                }
                Entry::File { .. } => {
                    if is_dir {
                        Err(libc::ENOTDIR)
                    } else {
                        Ok(())
                    }
                }
            }
        } else {
            Err(libc::ENOENT)
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
        self.check_entry(path, true)?;

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

    fn open(&self, req: RequestInfo, path: &Path, _flags: u32) -> ResultOpen {
        info!("{}: open {}", req.unique, path.to_string_lossy());
        self.rlock(req, |this| {
            this.check_entry(path, false)?;
            Ok((0, 0))
        })
    }

    fn create(
        &self,
        req: RequestInfo,
        parent: &Path,
        name: &OsStr,
        mode: u32,
        flags: u32,
    ) -> ResultCreate {
        info!(
            "{}: create {},{},{},{}",
            req.unique,
            parent.to_string_lossy(),
            name.to_string_lossy(),
            mode,
            flags,
        );
        self.wlock(req, |this| this.create(parent, name))
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

    fn utimens(
        &self,
        req: RequestInfo,
        path: &Path,
        _fh: Option<u64>,
        _atime: Option<Timespec>,
        mtime: Option<Timespec>,
    ) -> ResultEmpty {
        info!(
            "{}: utimens {}, {}",
            req.unique,
            path.to_string_lossy(),
            mtime
                .map(time::at_utc)
                .unwrap_or_else(time::empty_tm)
                .rfc3339(),
        );
        self.wlock(req, |this| this.utimens(path, mtime))
    }

    fn unlink(&self, req: RequestInfo, parent: &Path, name: &OsStr) -> ResultEmpty {
        info!(
            "{}: unlink {}, {}",
            req.unique,
            parent.to_string_lossy(),
            name.to_string_lossy()
        );
        self.wlock(req, |this| this.unlink(parent, name))
    }

    fn rmdir(&self, req: RequestInfo, parent: &Path, name: &OsStr) -> ResultEmpty {
        info!(
            "{}: rmdir {}, {}",
            req.unique,
            parent.to_string_lossy(),
            name.to_string_lossy()
        );
        self.wlock(req, |this| this.rmdir(parent, name))
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
            this.check_entry(path, true)?;
            Ok((0, 0))
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
