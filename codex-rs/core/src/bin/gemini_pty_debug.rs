//! Gemini PTY Debug Harness
//!
//! Interactive testing tool for Gemini PTY wrapper.
//! Allows manual testing of PTY communication, prompt detection, and streaming.
//!
//! Usage:
//!   cargo run --bin gemini_pty_debug
//!
//! Then type messages and see how the PTY wrapper handles them.
//! Special commands:
//!   /quit - Exit
//!   /compress - Compress conversation context
//!   /chat save <id> - Save conversation checkpoint
//!   /chat resume <id> - Resume from checkpoint

use codex_core::cli_executor::{GeminiPtySession, StreamEvent};
use std::io::{self, Write};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Note: Set RUST_LOG=debug for detailed logging
    // tracing is already initialized by codex_core

    println!("╔════════════════════════════════════════════════╗");
    println!("║   Gemini PTY Debug Harness                     ║");
    println!("╚════════════════════════════════════════════════╝");
    println!();
    println!("Model: gemini-2.5-flash");
    println!("Mode: Interactive PTY");
    println!();
    println!("Commands:");
    println!("  /quit           - Exit harness");
    println!("  /compress       - Compress context");
    println!("  /chat save <id> - Save checkpoint");
    println!("  /stats          - Show session stats");
    println!();
    println!("Type messages to send to Gemini CLI.");
    println!("Press Ctrl+D to quit.");
    println!("─────────────────────────────────────────────────");
    println!();

    // Create session
    let mut session = GeminiPtySession::new("gemini-2.5-flash");

    println!("[Starting Gemini CLI session...]");
    match session.start().await {
        Ok(_) => println!("✅ Session started successfully\n"),
        Err(e) => {
            eprintln!("❌ Failed to start session: {}", e);
            eprintln!("\nMake sure:");
            eprintln!("  1. Gemini CLI is installed: npm install -g @google/gemini-cli");
            eprintln!("  2. You're authenticated: Run 'gemini' and complete OAuth");
            return Err(e.into());
        }
    }

    // Read from stdin
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line_buffer = String::new();

    loop {
        // Show prompt
        print!("> ");
        io::stdout().flush()?;

        // Read line
        line_buffer.clear();
        let bytes_read = reader.read_line(&mut line_buffer).await?;

        if bytes_read == 0 {
            // EOF (Ctrl+D)
            println!("\n[EOF detected, shutting down...]");
            break;
        }

        let line = line_buffer.trim();

        if line.is_empty() {
            continue;
        }

        // Handle special commands
        match line {
            "/quit" => {
                println!("[Shutting down...]");
                break;
            }
            "/stats" => {
                let stats = session.stats();
                println!("\n╔════════════════════════════════════════════════╗");
                println!("║   Session Stats                                 ║");
                println!("╠════════════════════════════════════════════════╣");
                println!("  Turn count: {}", stats.turn_count);
                println!("  Last checkpoint: {:?}", stats.last_checkpoint);
                println!("  Conversation ID: {:?}", stats.conversation_id);
                println!("╚════════════════════════════════════════════════╝\n");
                continue;
            }
            _ if line.starts_with("/") => {
                // Pass CLI commands directly
                println!("[Sending CLI command: {}]", line);
                match session.send_command(line).await {
                    Ok(_) => println!("✅ Command sent\n"),
                    Err(e) => eprintln!("❌ Command failed: {}\n", e),
                }
                continue;
            }
            _ => {
                // Regular user message
            }
        }

        println!("[Sending message...]");
        println!();

        // Create channel for streaming
        let (tx, mut rx) = mpsc::channel(100);

        // Send message and stream in current task (keep session ownership)
        let send_future = session.send_message(line, tx);

        // Stream output to console concurrently
        print!("Gemini: ");
        io::stdout().flush()?;

        let mut response_started = false;
        let stream_task = tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    StreamEvent::Delta(text) => {
                        if !response_started {
                            response_started = true;
                        }
                        print!("{}", text);
                        io::stdout().flush().ok();
                    }
                    StreamEvent::Metadata(meta) => {
                        println!("\n[Metadata: model={}, in={:?}, out={:?}]",
                            meta.model, meta.input_tokens, meta.output_tokens);
                    }
                    StreamEvent::Done => {
                        println!("\n");
                        break;
                    }
                    StreamEvent::Error(e) => {
                        eprintln!("\n❌ Error: {}", e);
                        break;
                    }
                }
            }
        });

        // Wait for send to complete
        match send_future.await {
            Ok(response) => {
                println!("[Response complete: {} chars]", response.len());
            }
            Err(e) => {
                eprintln!("❌ Send failed: {}", e);
            }
        }

        // Wait for streaming to finish
        stream_task.await?;

        println!("─────────────────────────────────────────────────");
        println!();
    }

    // Shutdown
    println!("[Shutting down Gemini CLI session...]");
    match session.shutdown().await {
        Ok(_) => println!("✅ Shutdown complete"),
        Err(e) => eprintln!("⚠️  Shutdown error: {}", e),
    }

    Ok(())
}
