use std::io::{self, Write};

/// Prints a standard prompt for the server CLI.
pub fn print_prompt() {
    print!("\nmorpheus> ");
    io::stdout().flush().unwrap();
}

/// Prints a system message to the console.
pub fn print_system_message(msg: &str) {
    println!("\n[SYSTEM] {}\n", msg);
    print_prompt();
}

/// Prints an error message to the console.
pub fn print_error(msg: &str) {
    eprintln!("\n[ERROR] {}\n", msg);
    print_prompt();
}

/// Prints a confirmation of a sent message.
pub fn print_confirmation(msg: &str) {
    println!("\n[SENT] {}\n", msg);
    print_prompt();
}
