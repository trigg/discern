use std::env;
use std::path::Path;
use std::process::Command;

fn run_command(working_directory: &Path, executable_name: &str) {
    let exec = working_directory.join(executable_name);
    if exec.exists() {
        let mut command = Command::new(exec.clone());
        println!("{:?}", exec);
        command.output().expect("Failed to start");
    } else {
        println!("Unable to start {:?}", exec);
    }
}

#[tokio::main]
async fn main() {
    let wayland_env = env::var("WAYLAND_DISPLAY");
    let x11_env = env::var("DISPLAY");
    let gamescope_env = env::var("GAMESCOPE_WAYLAND_DISPLAY");
    let statefile_env = env::var("DISCERN_STATEFILE");

    match env::current_exe() {
        Ok(executable_path) => {
            println!("{:?}", executable_path);
            let working_directory = executable_path
                .parent()
                .expect("Unable to find working directory");
            println!("{:?}", working_directory);
            if statefile_env.is_ok() {
                run_command(working_directory, "discern-statefile");
            } else if gamescope_env.is_ok() {
                run_command(working_directory, "discern-gamescope");
            } else if wayland_env.is_ok() {
                run_command(working_directory, "discern-wlr");
            } else if x11_env.is_ok() {
                run_command(working_directory, "discern-x11");
            } else {
                run_command(working_directory, "discern-clispam");
            }
        }
        Err(e) => println!("Unable to find current executable location: {}", e),
    }
}
