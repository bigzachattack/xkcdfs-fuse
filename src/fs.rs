use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, Request,
};
use libc::{EINVAL, ENOENT};
use std::ffi::OsStr;
use std::time::{Duration, UNIX_EPOCH};

const TTL: Duration = Duration::from_secs(1);

const DIR_ATTR: FileAttr = FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH,
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o555,
    nlink: 2,
    uid: 1000,
    gid: 1000,
    rdev: 0,
    flags: 0,
    blksize: 512,
};

const XKCD_DESKTOP_CONTENT: &str = "[Desktop Entry]\nType=Link\nName=XKCD\nURL=https://xkcd.com/\n";
const XKCD_DESKTOP_ATTR: FileAttr = FileAttr {
    ino: 2,
    size: XKCD_DESKTOP_CONTENT.len() as u64,
    blocks: 1,
    atime: UNIX_EPOCH,
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::RegularFile,
    perm: 0o444,
    nlink: 1,
    uid: 1000,
    gid: 1000,
    rdev: 0,
    flags: 0,
    blksize: 512,
};

const ABOUT_CONTENT: &str =
    "xkcdfs-fuse is a unaffilated fan project for looking at XKCD comics.\n\n
xkcd is by Randall Munroe.\n";
const ABOUT_ATTR: FileAttr = FileAttr {
    ino: 3,
    size: ABOUT_CONTENT.len() as u64,
    blocks: 1,
    atime: UNIX_EPOCH,
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::RegularFile,
    perm: 0o444,
    nlink: 1,
    uid: 1000,
    gid: 1000,
    rdev: 0,
    flags: 0,
    blksize: 512,
};

const SUBDIR_ATTR: FileAttr = FileAttr {
    ino: 100,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH,
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o555,
    nlink: 2,
    uid: 1000,
    gid: 1000,
    rdev: 0,
    flags: 0,
    blksize: 512,
};

pub struct XkcdFs {
    pub latest_title: String,
    pub latest_alt: String,
    pub latest_img: Vec<u8>,
}

impl XkcdFs {
    fn create_file_attr(&self, ino: u64, size: u64) -> FileAttr {
        FileAttr {
            ino,
            size,
            blocks: 1,
            atime: UNIX_EPOCH,
            mtime: UNIX_EPOCH,
            ctime: UNIX_EPOCH,
            crtime: UNIX_EPOCH,
            kind: FileType::RegularFile,
            perm: 0o444,
            nlink: 1,
            uid: 1000,
            gid: 1000,
            rdev: 0,
            flags: 0,
            blksize: 512,
        }
    }

    fn get_file_attr(&self, ino: u64) -> Result<FileAttr, i32> {
        match ino {
            1 => Ok(DIR_ATTR),
            2 => Ok(XKCD_DESKTOP_ATTR),
            3 => Ok(ABOUT_ATTR),
            4 => Ok(self.create_file_attr(4, self.latest_title.len() as u64)),
            5 => Ok(self.create_file_attr(5, self.latest_alt.len() as u64)),
            6 => Ok(self.create_file_attr(6, self.latest_img.len() as u64)),
            100 => Ok(SUBDIR_ATTR),
            _ => Err(ENOENT),
        }
    }

    fn read_data(&self, ino: u64, offset: i64, size: u32) -> Result<&[u8], i32> {
        let data = match ino {
            2 => XKCD_DESKTOP_CONTENT.as_bytes(),
            3 => ABOUT_CONTENT.as_bytes(),
            4 => self.latest_title.as_bytes(),
            5 => self.latest_alt.as_bytes(),
            6 => &self.latest_img,
            _ => return Err(ENOENT),
        };

        if offset < 0 {
            return Err(EINVAL);
        }
        if offset as u64 >= data.len() as u64 {
            return Ok(b"");
        }

        let offset = offset as usize;
        let size = size as usize;
        let end = std::cmp::min(offset.saturating_add(size), data.len());
        Ok(&data[offset..end])
    }
}

impl Filesystem for XkcdFs {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let name = name.to_str();
        let ino = match (parent, name) {
            (1, Some("latest")) => 100,
            (1, Some("xkcd.desktop")) => 2,
            (1, Some("about.txt")) => 3,
            (100, Some("title.txt")) => 4,
            (100, Some("alt.txt")) => 5,
            (100, Some("image.png")) => 6,
            _ => {
                reply.error(ENOENT);
                return;
            }
        };

        match self.get_file_attr(ino) {
            Ok(attr) => reply.entry(&TTL, &attr, 0),
            Err(e) => reply.error(e),
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        match self.get_file_attr(ino) {
            Ok(attr) => reply.attr(&TTL, &attr),
            Err(e) => reply.error(e),
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        match self.read_data(ino, offset, size) {
            Ok(data) => reply.data(data),
            Err(e) => reply.error(e),
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let entries = if ino == 1 {
            vec![
                (1, FileType::Directory, "."),
                (1, FileType::Directory, ".."),
                (2, FileType::RegularFile, "xkcd.desktop"),
                (3, FileType::RegularFile, "about.txt"),
                (100, FileType::Directory, "latest"),
            ]
        } else if ino == 100 {
            vec![
                (100, FileType::Directory, "."),
                (1, FileType::Directory, ".."),
                (4, FileType::RegularFile, "title.txt"),
                (5, FileType::RegularFile, "alt.txt"),
                (6, FileType::RegularFile, "image.png"),
            ]
        } else {
            reply.error(ENOENT);
            return;
        };

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                break;
            }
        }
        reply.ok();
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;

    #[test]
    fn test_get_file_attr() {
        let fs = XkcdFs {
            latest_title: "Test Title".to_string(),
            latest_alt: "Test Alt".to_string(),
            latest_img: vec![1, 2, 3, 4],
        };

        assert_eq!(fs.get_file_attr(1).unwrap().kind, FileType::Directory);
        assert_eq!(fs.get_file_attr(2).unwrap().kind, FileType::RegularFile);
        assert_eq!(fs.get_file_attr(4).unwrap().size, 10); // "Test Title"
        assert_eq!(fs.get_file_attr(6).unwrap().size, 4);
        assert_eq!(fs.get_file_attr(999).unwrap_err(), ENOENT);
    }

    #[test]
    fn test_read_data() {
        let fs = XkcdFs {
            latest_title: "Title".to_string(),
            latest_alt: "Alt".to_string(),
            latest_img: vec![10, 20, 30],
        };

        // Test reading title (ino 4)
        let data = fs.read_data(4, 0, 100).unwrap();
        assert_eq!(data, b"Title");

        // Test offset
        let data = fs.read_data(4, 1, 100).unwrap();
        assert_eq!(data, b"itle");

        // Test size limit
        let data = fs.read_data(4, 0, 2).unwrap();
        assert_eq!(data, b"Ti");

        // Test offset past end
        let data = fs.read_data(4, 100, 10).unwrap();
        assert_eq!(data, b"");

        // Test negative offset
        let err = fs.read_data(4, -1, 10).unwrap_err();
        assert_eq!(err, EINVAL);

        // Test unknown inode
        let err = fs.read_data(999, 0, 10).unwrap_err();
        assert_eq!(err, ENOENT);
    }
}
