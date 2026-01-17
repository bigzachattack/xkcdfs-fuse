use clap::Parser;
use fuser::MountOption;
use serde::Deserialize;

mod fs;
use fs::XkcdFs;

#[derive(Parser)]
#[command(name = "xkcdfs-fuse")]
#[command(about = "XKCD FUSE filesystem")]
struct Args {
    /// Run in foreground
    #[arg(short, long)]
    foreground: bool,

    /// Mountpoint path
    mountpoint: std::path::PathBuf,
}

#[derive(Deserialize)]
struct XkcdComic {
    title: String,
    alt: String,
    img: String,
}

fn main() {
    let args = Args::parse();

    // Resolve the absolute path of the mountpoint before we daemonize and chdir
    let mountpoint = std::fs::canonicalize(&args.mountpoint).expect("Failed to resolve mountpoint");

    if !args.foreground {
        // Use libc::daemon to background the process.
        // First arg 0: change dir to /
        // Second arg 0: redirect stdio to /dev/null
        unsafe {
            if libc::daemon(0, 0) != 0 {
                eprintln!("Error daemonizing: {}", std::io::Error::last_os_error());
                std::process::exit(1);
            }
        }
    }

    let options = vec![MountOption::RO, MountOption::FSName("xkcdfs".to_string())];

    let comic: XkcdComic = reqwest::blocking::get("https://xkcd.com/info.0.json")
        .expect("Failed to fetch latest comic info")
        .json::<XkcdComic>()
        .expect("Failed to parse comic info");

    let image_bytes = reqwest::blocking::get(&comic.img)
        .expect("Failed to fetch comic image")
        .bytes()
        .expect("Failed to read image bytes")
        .to_vec();

    let fs = XkcdFs {
        latest_title: comic.title,
        latest_alt: comic.alt,
        latest_img: image_bytes,
    };
    fuser::mount2(fs, mountpoint, &options).unwrap();
}
