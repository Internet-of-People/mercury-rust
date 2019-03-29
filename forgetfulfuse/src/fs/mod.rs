mod imp;

use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::sync::*;

use fuse_mt::*;
use libc;
use log::*;
use time::{self, Timespec};
use timer::Timer;

use imp::*;

pub trait IgnoreResult: Sized {
    fn ignore(self) {}
}

impl<T, E> IgnoreResult for Result<T, E> {}

pub struct ForgetfulFS {
    inner: Arc<RwLock<FsImpl>>,
    timer: Arc<Mutex<Timer>>,
}

impl ForgetfulFS {
    pub fn new(uid: u32, gid: u32) -> Self {
        let inner = Arc::new(RwLock::new(FsImpl::new(uid, gid)));
        let timer = Arc::new(Mutex::new(Timer::with_capacity(128)));
        Self { inner, timer }
    }

    fn rlock<T, F>(&self, req: RequestInfo, f: F) -> Result<T, LibCError>
    where
        F: FnOnce(&RwLockReadGuard<'_, FsImpl>) -> Result<T, LibCError>,
    {
        Self::rlock_s(&self.inner, req, f)
    }

    fn wlock<T, F>(&self, req: RequestInfo, f: F) -> Result<T, LibCError>
    where
        F: FnOnce(&mut RwLockWriteGuard<'_, FsImpl>) -> Result<T, LibCError>,
    {
        Self::wlock_s(&self.inner, req, f)
    }

    fn rlock_s<T, F>(inner: &RwLock<FsImpl>, req: RequestInfo, f: F) -> Result<T, LibCError>
    where
        F: FnOnce(&RwLockReadGuard<'_, FsImpl>) -> Result<T, LibCError>,
    {
        trace!("{}: Trying to acquire read lock", req.unique);
        match inner.read() {
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

    fn wlock_s<T, F>(inner: &RwLock<FsImpl>, req: RequestInfo, f: F) -> Result<T, LibCError>
    where
        F: FnOnce(&mut RwLockWriteGuard<'_, FsImpl>) -> Result<T, LibCError>,
    {
        trace!("{}: Trying to acquire write lock", req.unique);
        match inner.write() {
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

    fn schedule_unlink(&self, req: RequestInfo, parent: PathBuf, name: OsString) -> ResultEmpty {
        let inner = self.inner.clone();
        match self.timer.lock() {
            Err(_e) => Err(libc::EAGAIN),
            Ok(timer) => {
                timer
                    .schedule_with_delay(time::Duration::seconds(5), {
                        let parent = parent.to_owned();
                        let name = name.to_owned();
                        move || {
                            Self::wlock_s(&inner, req, |this| this.unlink(&parent, &name)).ignore();
                        }
                    })
                    .ignore();
                Ok(())
            }
        }
    }
}

impl FilesystemMT for ForgetfulFS {
    fn init(&self, req: RequestInfo) -> ResultEmpty {
        info!("{}: init {}:{} from PID {}", req.unique, req.uid, req.gid, req.pid);
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
        self.schedule_unlink(req, parent.to_owned(), name.to_owned())?;
        self.wlock(req, |this| this.create(parent, name))
    }

    fn truncate(&self, req: RequestInfo, path: &Path, _fh: Option<u64>, size: u64) -> ResultEmpty {
        info!("{}: truncate {}, {}", req.unique, path.to_string_lossy(), size);
        self.wlock(req, |this| this.truncate(path, size))
    }

    fn read(&self, req: RequestInfo, path: &Path, _fh: u64, offset: u64, size: u32) -> ResultData {
        info!("{}: read {}, {}, {}", req.unique, path.to_string_lossy(), offset, size);
        self.rlock(req, |this| this.read(path, offset, size))
    }

    fn write(
        &self,
        req: RequestInfo,
        path: &Path,
        _fh: u64,
        offset: u64,
        data: Vec<u8>,
        _flags: u32,
    ) -> ResultWrite {
        info!("{}: write {}, {}, {}", req.unique, path.to_string_lossy(), offset, data.len());
        self.wlock(req, |this| this.write(path, offset, data))
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
            mtime.map(time::at_utc).unwrap_or_else(time::empty_tm).rfc3339(),
        );
        self.wlock(req, |this| this.utimens(path, mtime))
    }

    fn unlink(&self, req: RequestInfo, parent: &Path, name: &OsStr) -> ResultEmpty {
        info!("{}: unlink {}, {}", req.unique, parent.to_string_lossy(), name.to_string_lossy());
        self.wlock(req, |this| this.unlink(parent, name))
    }

    fn rmdir(&self, req: RequestInfo, parent: &Path, name: &OsStr) -> ResultEmpty {
        info!("{}: rmdir {}, {}", req.unique, parent.to_string_lossy(), name.to_string_lossy());
        self.wlock(req, |this| this.rmdir(parent, name))
    }

    fn mkdir(&self, req: RequestInfo, parent: &Path, name: &OsStr, _mode: u32) -> ResultEntry {
        info!("{}: mkdir {}, {}", req.unique, parent.to_string_lossy(), name.to_string_lossy());
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
