pub fn spawn(log: &slog::Logger, command: impl Into<String>) {
    let command = command.into();
    let parts = command.split_whitespace().collect::<Vec<_>>();

    if let Err(e) = std::process::Command::new(parts[0])
        .args(&parts[1..])
        .spawn()
    {
        slog::error!(log,
            "Failed to start program";
            "cmd" => command,
            "err" => format!("{:?}", e)
        );
    }
}
