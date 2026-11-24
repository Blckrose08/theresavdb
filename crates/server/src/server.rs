use crate::config::ServerConfig;
use std::sync::Arc;
use theresav_embedded::TheresavDB;
use theresav_error::TheresavError;
use theresav_protocol::{Frame, MessageCodec, MessageType};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub struct Server {
    config: ServerConfig,
    engine: Arc<TheresavDB>,
}

impl Server {
    pub fn new(config: ServerConfig) -> Result<Self, TheresavError> {
        let engine = Arc::new(TheresavDB::open(&config.data_dir)?);
        Ok(Self { config, engine })
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let bind_addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&bind_addr).await?;

        println!("TheresavDB Server running at tcp://{}", bind_addr);

        loop {
            let (socket, client_addr) = listener.accept().await?;
            let engine = self.engine.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_connection(socket, engine).await {
                    eprintln!("Connection error from {}: {}", client_addr, e);
                }
            });
        }
    }
}

async fn handle_connection(
    mut socket: TcpStream,
    engine: Arc<TheresavDB>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut read_buf = vec![0u8; 64 * 1024];

    loop {
        let n = socket.read(&mut read_buf).await?;
        if n == 0 {
            break;
        }

        let slice = &read_buf[..n];
        let (frame, _) = match Frame::decode(slice) {
            Ok(f) => f,
            Err(e) => {
                let err_frame = MessageCodec::encode_error(0, "PROTOCOL_ERR", &e.to_string());
                socket.write_all(&err_frame.encode()).await?;
                continue;
            }
        };

        match frame.header.message_type {
            MessageType::Ping => {
                let pong = Frame::new(MessageType::Pong, frame.header.request_id, vec![]);
                socket.write_all(&pong.encode()).await?;
            }
            MessageType::Query => {
                let query_msg = MessageCodec::decode_query(&frame)?;
                match engine.execute(&query_msg.sql) {
                    Ok(result) => {
                        if !result.columns.is_empty() {
                            let desc = MessageCodec::encode_row_desc(
                                frame.header.request_id,
                                result.columns,
                            );
                            socket.write_all(&desc.encode()).await?;
                        }

                        for row in result.rows {
                            let data_row =
                                MessageCodec::encode_data_row(frame.header.request_id, row);
                            socket.write_all(&data_row.encode()).await?;
                        }

                        let complete = MessageCodec::encode_complete(
                            frame.header.request_id,
                            result.rows_affected,
                        );
                        socket.write_all(&complete.encode()).await?;
                    }
                    Err(e) => {
                        let err_frame = MessageCodec::encode_error(
                            frame.header.request_id,
                            "EXEC_ERR",
                            &e.to_string(),
                        );
                        socket.write_all(&err_frame.encode()).await?;
                    }
                }
            }
            MessageType::Close => {
                break;
            }
            _ => {
                let err_frame = MessageCodec::encode_error(
                    frame.header.request_id,
                    "UNSUPPORTED",
                    "Unsupported message type in this session state",
                );
                socket.write_all(&err_frame.encode()).await?;
            }
        }
    }

    Ok(())
}
