//! Gemini PTY Debug REPL (SPEC-952-F)
//!
//! Standalone REPL for testing PTY sessions outside the TUI.
//! Useful for debugging multi-turn conversations and PTY behavior.
//!
//! Usage:
//!   cargo run --bin gemini_pty_debug
//!
//! Commands:
//!   /quit     - Exit the REPL
//!   /help     - Show this help
//!   /restart  - Restart the PTY session
//!   anything else - Send as message to Gemini

use codex_core::cli_executor::gemini_pty::{GeminiPtySession, GeminiPtyConfig};
use codex_core::cli_executor::StreamEvent;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Gemini PTY Debug REPL");
    println!("Commands: /quit, /help, /restart");
    println!("Type your messages below:\n");

    // Run REPL in blocking task
    tokio::task::spawn_blocking(|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let config = GeminiPtyConfig::default();
        let mut session = GeminiPtySession::new(&config.model);

        println!("Starting Gemini PTY session with model: {}", config.model);
        session.start()?;
        println!("Session started successfully\n");

        loop {
            // Read user input
            print!("> ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            // Handle commands
            match input {
                "/quit" => {
                    println!("Shutting down...");
                    break;
                }
                "/help" => {
                    println!("Commands:");
                    println!("  /quit     - Exit the REPL");
                    println!("  /help     - Show this help");
                    println!("  /restart  - Restart the PTY session");
                    println!("  anything else - Send as message to Gemini\n");
                    continue;
                }
                "/restart" => {
                    println!("Restarting session...");
                    session.shutdown()?;
                    session = GeminiPtySession::new(&config.model);
                    session.start()?;
                    println!("Session restarted\n");
                    continue;
                }
                "" => continue, // Empty input
                _ => {
                    // Send message to Gemini
                    let (tx, mut rx) = mpsc::channel(32);
                    let cancel = CancellationToken::new();

                    // Spawn async task to print streaming output
                    let handle = tokio::runtime::Handle::current();
                    let stream_task = handle.spawn(async move {
                        while let Some(event) = rx.recv().await {
                            match event {
                                StreamEvent::Delta(text) => {
                                    print!("{}", text);
                                    io::stdout().flush().ok();
                                }
                                StreamEvent::Done => {
                                    println!("\n");
                                    break;
                                }
                                StreamEvent::Error(e) => {
                                    eprintln!("\n[ERROR: {}]\n", e);
                                    break;
                                }
                                _ => {}
                            }
                        }
                    });

                    // Send message (blocking) - returns accumulated response
                    match session.send_message(input, tx, cancel) {
                        Ok(response) => {
                            // Wait for streaming to complete
                            futures::executor::block_on(stream_task).ok();
                            println!("[DONE - {} chars]\n", response.len());
                        }
                        Err(e) => {
                            eprintln!("Error sending message: {}\n", e);
                        }
                    }
                }
            }
        }

        // Clean shutdown
        session.shutdown()?;
        Ok(())
    })
    .await
    .map_err(|e| format!("Join error: {}", e))?
    .map_err(|e| format!("Session error: {}", e))?;

    Ok(())
}
