use std::{
    env,
    ffi::OsString,
    path::PathBuf,
    process::exit,
};

use glob::glob;

// Pseudocode
// Get all Neovim instances
// Get the working dirs of all instances
// Check the arguments
// Choose a Neovim instance:
//   If the file is under a directory in the running instances,
//      Open file in that instance
//   Else
//      Create a new instance
static NVR_PROGRAM: &str = "nvr --nostart -s";

fn main() {
    if let Some(paths) = get_nvim_socket_paths() {
        println!("Found paths! {:?}", paths)
    } else {
        eprintln!("Error: environment variables could not be found.");
        exit(1)
    }
}

fn get_nvim_socket_paths() -> Option<Vec<PathBuf>> {
    let mut paths = Vec::<PathBuf>::new();
    if let Some(path_glob) = get_nvim_glob() {
        for entry in path_glob {
            match entry {
                Ok(socket_path) => paths.push(socket_path),
                Err(err) => eprintln!("{:?}", err),
            }
        }
    } else {
        return None
    }

    Some(paths)
}

fn get_nvim_glob() -> Option<glob::Paths> {
    let mut path;
    if let Some(xdg_runtime_dir) = env::var_os("XDG_RUNTIME_DIR") {
        path = PathBuf::from(xdg_runtime_dir)
    } else if let Some(tmp_dir) = env::var_os("TMPDIR") {
        if let Some(user) = env::var_os("USER") {
            path = PathBuf::from(tmp_dir);
            let prefix = OsString::from("nvim.");
            let mut segment = OsString::with_capacity(prefix.len() + user.len());
            segment.push(prefix);
            segment.push(user);
            path.push(segment);
        } else {
            return None;
        }
    } else {
        return None;
    }

    path.push("*");
    path.push("nvim.*.0");

    let glob_str = path.to_str()?;

    Some(glob(glob_str).expect("Fatal: an invalid glob was created by get_nvim_glob."))
}
