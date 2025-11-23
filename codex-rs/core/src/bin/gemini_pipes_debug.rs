//! Debug tool for Gemini Pipes Session
//!
//! Interactive REPL for testing Gemini CLI pipes integration.
//! Useful for debugging without full TUI overhead.
//!
//! Usage:
//!   cargo run --bin gemini_pipes_debug
//!
//! Commands:
//!   <message>  - Send message to Gemini
//!   /quit      - Exit
//!   /stats     - Show session statistics
//!   /model <m> - Change model (creates new session)

use std::io::{self, Write};

use codex_core::cli_executor::StreamEvent;
use codex_core::cli_executor::gemini_pipes::GeminiPipesSession;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Note: For detailed logging, set RUST_LOG=debug before running

    println!("═══════════════════════════════════════════════════════");
    println!("  Gemini Pipes Debug Console");
    println!("═══════════════════════════════════════════════════════");
    println!("Commands:");
    println!("  <message> - Send message to Gemini");
    println!("  /quit     - Exit");
    println!("  /stats    - Show session statistics");
    println!("  /model <m> - Change model (creates new session)");
    println!("═══════════════════════════════════════════════════════\n");

    let mut model = "gemini-2.5-flash".to_string();
    let mut session = GeminiPipesSession::spawn(&model, None).await?;

    println!("✓ Session started (model: {})\n", model);

    loop {
        // Prompt
        print!("> ");
        io::stdout().flush()?;

        // Read line
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        // Handle commands
        if input == "/quit" {
            println!("\nShutting down...");
            session.shutdown().await?;
            break;
        } else if input == "/stats" {
            let stats = session.stats();
            println!("\n─── Session Statistics ───");
            println!("Model:       {}", stats.model);
            println!("Turn count:  {}", stats.turn_count);
            println!("Conv ID:     {:?}", stats.conversation_id);
            println!("──────────────────────────\n");
            continue;
        } else if input.starts_with("/model ") {
            let new_model = input.strip_prefix("/model ").unwrap().trim();
            println!("\nSwitching to model: {}", new_model);

            // Shutdown old session
            session.shutdown().await?;

            // Create new session
            model = new_model.to_string();
            session = GeminiPipesSession::spawn(&model, None).await?;
            println!("✓ New session started\n");
            continue;
        }

        // Send message
        println!();
        if let Err(e) = session.send_user_message(input).await {
            eprintln!("Error sending message: {}", e);
            continue;
        }

        // Stream response
        let (tx, mut rx) = tokio::sync::mpsc::channel(128);
        let cancel = CancellationToken::new();

        // Spawn stream task and print concurrently
        let stream_task =
            tokio::spawn(async move { session.stream_turn(tx, cancel).await.map(|_| session) });

        // Print response as it streams
        while let Some(event) = rx.recv().await {
            match event {
                StreamEvent::Delta(text) => {
                    print!("{}", text);
                    io::stdout().flush()?;
                }
                StreamEvent::Done => {
                    println!("\n");
                    break;
                }
                StreamEvent::Error(e) => {
                    eprintln!("\n[ERROR] {}\n", e);
                    break;
                }
                _ => {}
            }
        }

        // Wait for stream task to complete and get session back
        match stream_task.await {
            Ok(Ok(sess)) => {
                session = sess;
            }
            Ok(Err(e)) => {
                eprintln!("[ERROR] Stream error: {}. Session may be dead.\n", e);
                // Try to recreate session
                println!("Attempting to recreate session...");
                match GeminiPipesSession::spawn(&model, None).await {
                    Ok(new_session) => {
                        session = new_session;
                        println!("✓ Session recreated\n");
                    }
                    Err(e) => {
                        eprintln!("[FATAL] Cannot recreate session: {}", e);
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("[FATAL] Task panic: {}", e);
                break;
            }
        }
    }

    println!("Goodbye!");
    Ok(())
}
