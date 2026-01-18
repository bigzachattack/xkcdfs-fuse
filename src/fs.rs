use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, Request,
};
use libc::{EINVAL, ENOENT};
use serde::Deserialize;
use std::collections::HashMap;
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

#[derive(Deserialize, Default)]
struct XkcdComic {
    num: u64,
    year: String,
    month: String,
    day: String,
    title: String,
    alt: String,
    img: String,
    safe_title: String,
    transcript: String,
    link: String,
}

#[derive(Default)]
pub struct XkcdFs {
    latest_num: u64,
    http_client: reqwest::blocking::Client,
    comics: HashMap<u64, XkcdComic>,
}

const COMIC_INODE_SHIFT: u64 = 1000;

impl XkcdFs {
    fn get_latest_num(&mut self) -> u64 {
        if self.latest_num == 0 {
            self.get_latest_comic();
        }
        return self.latest_num;
    }

    fn get_latest_comic(&mut self) {
        let comic: XkcdComic = reqwest::blocking::get("https://xkcd.com/info.0.json")
            .expect("Failed to fetch latest comic info")
            .json::<XkcdComic>()
            .expect("Failed to parse comic info");

        self.latest_num = comic.num as u64;
        self.comics.insert(self.latest_num, comic);
    }

    fn inode_to_comic(&self, inode: u64) -> Option<&XkcdComic> {
        if inode > self.comics.len() as u64 {
            return None;
        }
        if inode < COMIC_INODE_SHIFT {
            return None;
        }
        let comic_num = inode / COMIC_INODE_SHIFT;
        return self.comics.get(&comic_num);
    }

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

    fn get_file_attr(&mut self, ino: u64) -> Result<FileAttr, i32> {
        match ino {
            1 => Ok(DIR_ATTR),
            2 => Ok(XKCD_DESKTOP_ATTR),
            3 => Ok(ABOUT_ATTR),
            100 => Ok(FileAttr {
                ino: 100,
                size: self.get_latest_num().to_string().len() as u64,
                blocks: 0,
                atime: UNIX_EPOCH,
                mtime: UNIX_EPOCH,
                ctime: UNIX_EPOCH,
                crtime: UNIX_EPOCH,
                kind: FileType::Symlink,
                perm: 0o444,
                nlink: 1,
                uid: 1000,
                gid: 1000,
                rdev: 0,
                flags: 0,
                blksize: 512,
            }),
            n if n % COMIC_INODE_SHIFT == 0 => Ok(FileAttr {
                ino: n,
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
            }),
            n if n % COMIC_INODE_SHIFT == 4 => Ok(self.create_file_attr(n, 4096)),
            n if n % COMIC_INODE_SHIFT == 5 => Ok(self.create_file_attr(n, 4096)),
            n if n % COMIC_INODE_SHIFT == 6 => Ok(self.create_file_attr(n, 4096)),
            _ => Err(ENOENT),
        }
    }

    fn read_data(&self, ino: u64, offset: i64, size: u32) -> Result<&[u8], i32> {
        let data = match ino {
            2 => XKCD_DESKTOP_CONTENT.as_bytes(),
            3 => ABOUT_CONTENT.as_bytes(),
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
        let name = match name.to_str() {
            Some(n) => n,
            None => {
                reply.error(EINVAL);
                return;
            }
        };
        let ino: u64;

        let comic_num = name
            .parse::<u64>()
            .ok() // Turns Result into Option (discards the error)
            .filter(|&n| (1..=self.get_latest_num()).contains(&n));

        if let Some(num) = comic_num {
            ino = num * COMIC_INODE_SHIFT;
        } else if parent % COMIC_INODE_SHIFT == 0 {
            ino = match name {
                "title.txt" => 4,
                "alt.txt" => 5,
                "image.png" => 6,
                _ => {
                    reply.error(ENOENT);
                    return;
                }
            };
        } else {
            ino = match (parent, name) {
                (1, "latest") => 100,
                (1, "xkcd.desktop") => 2,
                (1, "about.txt") => 3,

                _ => {
                    reply.error(ENOENT);
                    return;
                }
            };
        }

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

    fn readlink(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyData) {
        if ino == 100 {
            reply.data(self.get_latest_num().to_string().as_bytes());
            return;
        }
        reply.error(ENOENT);
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let mut entries: Vec<(u64, FileType, String)>;
        if ino == 1 {
            entries = vec![
                (1, FileType::Directory, ".".to_owned()),
                (1, FileType::Directory, "..".to_owned()),
                (2, FileType::RegularFile, "xkcd.desktop".to_owned()),
                (3, FileType::RegularFile, "about.txt".to_owned()),
                (100, FileType::Directory, "latest".to_owned()),
            ];
            for i in 1..self.get_latest_num() {
                entries.push((i * COMIC_INODE_SHIFT, FileType::Directory, i.to_string()));
            }
        } else if ino % COMIC_INODE_SHIFT == 0 {
            entries = vec![
                (ino, FileType::Directory, ".".to_owned()),
                (1, FileType::Directory, "..".to_owned()),
                (ino + 4, FileType::RegularFile, "title.txt".to_owned()),
                (ino + 5, FileType::RegularFile, "alt.txt".to_owned()),
                (ino + 6, FileType::RegularFile, "image.png".to_owned()),
            ];
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
        let mut fs = XkcdFs {
            latest_num: 0,
            http_client: reqwest::blocking::Client::new(),
            comics: vec![],
        };

        assert_eq!(fs.get_file_attr(1).unwrap().kind, FileType::Directory);
        assert_eq!(fs.get_file_attr(2).unwrap().kind, FileType::RegularFile);
        assert_eq!(fs.get_file_attr(3).unwrap().kind, FileType::RegularFile);
        assert_eq!(fs.get_file_attr(999).unwrap_err(), ENOENT);
    }

    #[test]
    fn test_read_data() {
        let fs = XkcdFs {
            latest_num: 0,
            http_client: reqwest::blocking::Client::new(),
            comics: vec![],
        };

        // Test reading desktop file (ino 2)
        let data = fs.read_data(2, 0, 100).unwrap();
        assert_eq!(data, XKCD_DESKTOP_CONTENT.as_bytes());

        // Test reading about file (ino 3)
        let data = fs.read_data(3, 0, 100).unwrap();
        assert_eq!(data, ABOUT_CONTENT.as_bytes());

        // Test unknown inode
        let err = fs.read_data(999, 0, 10).unwrap_err();
        assert_eq!(err, ENOENT);
    }
}
