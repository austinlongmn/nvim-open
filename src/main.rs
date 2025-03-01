use std::{
    ffi::OsString,
    os::unix::ffi::OsStringExt,
    path::PathBuf,
    process::exit,
    str::FromStr,
    time::Duration,
};
use std::env::{self};

use async_process::Command;
use async_std::{prelude::FutureExt, task};
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

#[derive(Debug, Clone)]
struct NeovimInstance {
    server_address: PathBuf,
    working_directory: PathBuf,
}

fn main() {
    if let Some(paths) = get_nvim_socket_paths() {
        println!("Found paths! {:?}", paths);
        let nvim_instances = get_nvim_instances(paths);
        println!("Here are the instances: {:?}", nvim_instances);
        println!(
            "Here is the instance for src/main.rs: {:?}",
            get_instance_for_path(
                &PathBuf::from_str("src/main.rs").expect("Invalid path"),
                nvim_instances
            )
        )
    } else {
        eprintln!("Error: environment variables could not be found.");
        exit(1)
    }
}

fn get_instance_for_path(
    path: &PathBuf,
    nvim_instances: Vec<NeovimInstance>,
) -> Option<NeovimInstance> {
    nvim_instances
        .into_iter()
        .find(|inst| path_under_directory(path, &inst.working_directory))
}

fn path_under_directory(path: &PathBuf, dir: &PathBuf) -> bool {
    let canonical_dir = dir.canonicalize();
    let canonical_path = path.canonicalize();

    if let Ok(canonical_dir) = canonical_dir {
        if let Ok(canonical_path) = canonical_path {
            canonical_path.starts_with(canonical_dir)
        } else {
            false
        }
    } else {
        false
    }
}

fn get_nvim_instances(addresses: Vec<PathBuf>) -> Vec<NeovimInstance> {
    task::block_on(get_nvim_instances_async(addresses))
}

async fn get_nvim_instances_async(addresses: Vec<PathBuf>) -> Vec<NeovimInstance> {
    let mut processes = Vec::with_capacity(addresses.len());

    async fn get_instance(address: PathBuf) -> Option<NeovimInstance> {
        let mut cmd = Command::new("nvr");
        cmd.arg("--nostart")
            .arg("-s")
            .arg("--servername")
            .arg(&address)
            .arg("--remote-expr")
            .arg("getcwd()");

        let cmd_results = cmd.output().timeout(Duration::from_secs(5)).await;

        match cmd_results {
            Ok(cmd_in_time_result) => match cmd_in_time_result {
                Ok(cmd_output) => {
                    if !cmd_output.status.success() {
                        eprintln!("{:?} exited with status code {}.", cmd, cmd_output.status)
                    }

                    let mut cmd_stdout = cmd_output.stdout;

                    if cmd_stdout.last() == Some(&b'\n') {
                        cmd_stdout.pop();
                    }

                    println!("cmd_stdout is {:?}", std::str::from_utf8(&cmd_stdout));

                    Some(NeovimInstance {
                        server_address: address,
                        working_directory: PathBuf::from(OsString::from_vec(cmd_stdout)),
                    })
                }
                Err(cmd_err) => {
                    eprintln!(
                        "An error occured when executing command {:?}: {}",
                        cmd, cmd_err
                    );

                    None
                }
            },
            Err(_) => {
                eprintln!(
                    "The command request to server at {} timed out.",
                    address.display()
                );

                None
            }
        }
    }

    for address in addresses {
        processes.push(task::spawn_local(get_instance(address)));
    }

    let mut result = Vec::with_capacity(processes.len());

    for process in processes {
        result.push(process.await);
    }

    result.into_iter().flatten().collect()
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
        return None;
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
