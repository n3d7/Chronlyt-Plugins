fn main() -> std::process::ExitCode {
    match chronlyt_registry::commands::run(std::env::args_os().skip(1).collect()) {
        Ok(()) => {
            println!("Registry operation completed.");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
