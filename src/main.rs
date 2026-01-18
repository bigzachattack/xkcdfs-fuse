use clap::Parser;
use fuser::MountOption;

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

    let fs = XkcdFs::default();
    fuser::mount2(fs, mountpoint, &options).unwrap();
}
