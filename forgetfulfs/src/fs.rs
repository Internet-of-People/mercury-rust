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
        let mtime_sec = Self::now_utc();
        entries.insert(PathBuf::from("/"), Entry::Dir { mtime_sec });
        entries.insert(PathBuf::from("/hello.txt"), Entry::File { mtime_sec, blob: b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Morbi vel sem in mi tempus facilisis. Sed dapibus est quis ante placerat, sit amet luctus erat ornare. Duis finibus orci risus, id volutpat dui consectetur nec. Nullam ultrices non sapien in hendrerit. Aenean sit amet massa risus. Donec sit amet lorem mauris. Donec sit amet tristique eros. Phasellus sit amet nulla elit. Cras nibh tellus, imperdiet vitae molestie quis, placerat pellentesque lorem. Vestibulum hendrerit ipsum ut est malesuada mattis. Donec elementum efficitur quam et tincidunt. Mauris malesuada nulla neque, id tempor ligula placerat in. Phasellus nulla elit, tristique ornare hendrerit et, sodales a purus. Cras hendrerit dui ac elit porta, id commodo mauris faucibus. Nullam porttitor suscipit varius.
Donec dictum, massa ac consequat imperdiet, ligula tellus varius justo, et pulvinar velit quam vitae mi. Maecenas ac lorem nunc. Aenean lacinia dignissim tortor placerat tincidunt. Ut non dui ut dolor auctor hendrerit. Aenean vitae dui et nisi aliquam molestie non in lorem. Morbi sollicitudin enim sit amet consequat dictum. Phasellus lobortis rutrum mi, ac ornare metus mattis sit amet. Donec nec fringilla odio. Donec vehicula, sem sit amet tincidunt porttitor, neque tellus blandit nibh, in sollicitudin mauris orci quis lacus. Donec elit felis, congue vel facilisis quis, sodales nec ipsum. Curabitur hendrerit leo lectus, pretium efficitur eros vulputate eget. Aenean bibendum dui ut ex pretium, eget feugiat lacus auctor. Curabitur ut erat lectus. Cras vitae augue blandit, tincidunt ex in, feugiat ipsum. Duis lacinia risus nunc, eget consequat massa vulputate elementum. Mauris at pellentesque leo, in mattis nisi.
Morbi eros nibh, tempus sit amet enim eu, finibus suscipit enim. Sed quis libero diam. Duis accumsan et purus at posuere. Nulla aliquam tincidunt ante, egestas bibendum nunc blandit at. Aliquam sit amet eleifend leo, vel gravida tellus. Donec lacus justo, rutrum vitae mattis vitae, congue nec ipsum. Sed varius lacinia est, sed scelerisque diam commodo in. Nullam consequat, purus eu dignissim suscipit, ex quam lacinia urna, posuere euismod turpis quam eget nulla.
Nullam blandit neque erat, eget placerat enim porttitor non. Nullam vel est at nunc tincidunt iaculis non eget ipsum. Donec tristique tempus consectetur. Quisque iaculis lectus ut odio vulputate, quis condimentum sapien pharetra. Phasellus ultricies lorem at neque ultrices, id varius diam imperdiet. Duis ultrices tellus non felis tempus molestie. Ut consectetur id arcu at gravida. Etiam mauris ipsum, ultrices et mollis at, mollis eget lectus. Donec posuere velit quis nibh rutrum tempus. Quisque erat nunc, pretium efficitur vulputate at, pharetra eu arcu. Sed erat lacus, iaculis eget purus a, vehicula bibendum mauris. Sed ultricies varius ligula, at tempor lacus vulputate lobortis. Suspendisse potenti.
Vestibulum sagittis est dolor, nec euismod massa rhoncus sed. Mauris vel arcu lobortis, aliquam leo a, elementum urna. Curabitur et nisl quis velit cursus condimentum ac id neque. Praesent rhoncus mi eget tellus malesuada maximus. Maecenas nec porta eros, id placerat mi. Cras a urna gravida, imperdiet eros in, lobortis tellus. Morbi viverra, odio eu malesuada iaculis, tellus felis posuere mi, quis aliquam dolor eros eget enim. Mauris lacinia volutpat ipsum. Fusce eget auctor mauris. In dolor ex, pharetra eu purus ac, auctor vestibulum purus. Donec ultricies mollis diam, et rhoncus est euismod a. Pellentesque ut metus non nulla luctus condimentum. Etiam quis lectus porta orci sagittis imperdiet.
".to_vec() });
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

    fn blob(&self, path: &Path) -> Result<&Blob, LibCError> {
        match self.entries.get(path).ok_or(libc::ENOENT)? {
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
        let entry = self.entries.get_mut(path).ok_or(libc::ENOENT)?;
        let x = match entry {
            Entry::Dir { mtime_sec } | Entry::File { mtime_sec, .. } => mtime_sec,
        };
        if mtime.is_some() {
            *x = mtime.unwrap().sec;
        }
        Ok(())
    }

    fn read(&self, path: &Path, offset: u64, size: u32) -> ResultData {
        let blob = self.blob(path)?;
        Ok(blob
            .iter()
            .skip(offset as usize)
            .take(size as usize)
            .cloned()
            .collect())
    }

    fn write(&mut self, path: &Path, offset: u64, data: Vec<u8>) -> ResultWrite {
        let blob = self.blob_mut(path)?;
        let count = data.len();
        let start = usize::min(offset as usize, blob.len());
        let end = usize::min(offset as usize + data.len(), blob.len());
        let _discarded: Vec<u8> = blob.splice(start..end, data).collect();
        Ok(count as u32)
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

    fn read(&self, req: RequestInfo, path: &Path, _fh: u64, offset: u64, size: u32) -> ResultData {
        info!(
            "{}: read {}, {}, {}",
            req.unique,
            path.to_string_lossy(),
            offset,
            size
        );
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
        info!(
            "{}: read {}, {}, {}",
            req.unique,
            path.to_string_lossy(),
            offset,
            data.len()
        );
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
