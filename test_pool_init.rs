// Temporary test to debug pool initialization
use std::path::PathBuf;

fn main() {
    let db_path = dirs::home_dir()
        .expect("HOME not found")
        .join(".code")
        .join("consensus_artifacts.db");

    println!("Attempting to initialize pool for: {:?}", db_path);
    println!("File exists: {}", db_path.exists());

    match codex_core::db::initialize_pool(&db_path, 10) {
        Ok(pool) => {
            println!("✅ Pool initialized successfully!");
            println!("Max size: {}", pool.max_size());

            match pool.get() {
                Ok(conn) => {
                    println!("✅ Got connection from pool");

                    // Try to run migrations
                    match codex_core::db::migrations::migrate_to_latest(&mut conn.clone()) {
                        Ok(_) => println!("✅ Migrations completed"),
                        Err(e) => println!("❌ Migration failed: {}", e),
                    }
                }
                Err(e) => println!("❌ Failed to get connection: {}", e),
            }
        }
        Err(e) => {
            println!("❌ Pool initialization failed: {}", e);
            println!("Error type: {:?}", e);
        }
    }
}
