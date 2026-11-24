//! Command-line interface for interacting with TheresavDB.

use std::env;
use std::path::PathBuf;
use theresav_embedded::TheresavDB;
use theresav_protocol::{Frame, MessageCodec, MessageType};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    // Check embedded mode
    if let Some(pos) = args.iter().position(|a| a == "--embedded") {
        let dir = args.get(pos + 1).cloned().unwrap_or_else(|| "./data".to_string());
        println!("Connecting to embedded TheresavDB at '{}'...", dir);
        let db = TheresavDB::open(&PathBuf::from(dir))?;

        if let Some(cmd_pos) = args.iter().position(|a| a == "-c") {
            if let Some(sql) = args.get(cmd_pos + 1) {
                for statement in sql.split(';') {
                    let trimmed = statement.trim();
                    if !trimmed.is_empty() {
                        let query_with_semi = format!("{};", trimmed);
                        let res = db.execute(&query_with_semi)?;
                        print_result(&res.columns, &res.rows, res.rows_affected);
                    }
                }
                return Ok(());
            }
        }

        // Run default interactive banner
        println!("TheresavDB v0.1.0 (Embedded Mode)");
        println!("Type SQL queries ending with ';' or 'exit' to quit.\n");
        return Ok(());
    }

    // Default: TCP Client
    let host = "127.0.0.1";
    let port = 7432;
    let target = format!("{}:{}", host, port);

    if let Some(cmd_pos) = args.iter().position(|a| a == "-c") {
        if let Some(sql) = args.get(cmd_pos + 1) {
            let mut stream = TcpStream::connect(&target).await?;
            let frame = MessageCodec::encode_query(1, sql);
            stream.write_all(&frame.encode()).await?;

            let mut buf = vec![0u8; 64 * 1024];
            let n = stream.read(&mut buf).await?;
            let (resp, _) = Frame::decode(&buf[..n])?;

            if resp.header.message_type == MessageType::Error {
                eprintln!("Error response: {:?}", resp);
            } else {
                println!("Response received successfully.");
            }
            return Ok(());
        }
    }

    println!("TheresavDB CLI v0.1.0");
    println!("Usage:");
    println!("  theresav-cli --embedded <data_dir> -c \"<sql>\"");
    println!("  theresav-cli --host 127.0.0.1 --port 7432 -c \"<sql>\"");

    Ok(())
}

fn print_result(columns: &[String], rows: &[Vec<theresav_types::Value>], rows_affected: u64) {
    if !columns.is_empty() {
        println!("{}", columns.join(" | "));
        println!("{}", "-".repeat(columns.len() * 16));
        for row in rows {
            let row_str: Vec<String> = row.iter().map(|v| v.to_string()).collect();
            println!("{}", row_str.join(" | "));
        }
    }
    println!("({} rows affected)\n", rows_affected);
}
