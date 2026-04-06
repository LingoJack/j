use j_cli::core::executor;

#[tauri::command]
pub fn execute_command(input: String) -> executor::CommandResult {
    executor::execute_command(&input)
}
