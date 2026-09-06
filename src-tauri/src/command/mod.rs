use serde::Serialize;
use std::process::Command;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub fn execute_command(command: &str) -> CommandResult {
    let output = platform_command(command).output();

    match output {
        Ok(output) => CommandResult {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        },
        Err(error) => CommandResult {
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: error.to_string(),
        },
    }
}

#[cfg(target_os = "windows")]
fn platform_command(command: &str) -> Command {
    let mut process = Command::new("powershell.exe");
    let script = format!(
        "[Console]::OutputEncoding = [Text.UTF8Encoding]::new(); [Console]::InputEncoding = [Text.UTF8Encoding]::new(); {command}"
    );
    process.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        &script,
    ]);
    process
}

#[cfg(target_os = "macos")]
fn platform_command(command: &str) -> Command {
    let mut process = Command::new("/bin/zsh");
    process.args(["-c", command]);
    process
}

#[cfg(target_os = "linux")]
fn platform_command(command: &str) -> Command {
    let mut process = Command::new("/bin/sh");
    process.args(["-c", command]);
    process
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn supports_quotes_pipes_environment_and_multiple_commands() {
        let result = execute_command(
            "MULTIBOOT_VALUE='hello world'; printf '%s' \"$MULTIBOOT_VALUE\" | tr a-z A-Z",
        );
        assert!(result.success, "{}", result.stderr);
        assert_eq!(result.stdout, "HELLO WORLD");
        assert_eq!(result.exit_code, Some(0));
    }

    #[test]
    fn captures_failure_status_and_stderr() {
        #[cfg(target_os = "windows")]
        let command = "[Console]::Error.Write('expected error'); exit 7";
        #[cfg(not(target_os = "windows"))]
        let command = "printf 'expected error' >&2; exit 7";

        let result = execute_command(command);
        assert!(!result.success);
        assert_eq!(result.exit_code, Some(7));
        assert_eq!(result.stderr, "expected error");
    }
}
