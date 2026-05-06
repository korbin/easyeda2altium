use easyeda2altium::cli::run;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("[easyeda2altium] error: {}", e);
            std::process::ExitCode::FAILURE
        }
    }
}
