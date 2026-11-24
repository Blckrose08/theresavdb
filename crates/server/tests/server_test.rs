use std::sync::Arc;
use tempfile::tempdir;
use theresav_protocol::{Frame, MessageCodec, MessageType};
use theresav_server::{Server, ServerConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::test]
async fn test_tcp_server_end_to_end_query() {
    let dir = tempdir().unwrap();
    let port = 17432;

    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port,
        max_connections: 10,
        max_frame_size: 16 * 1024 * 1024,
        data_dir: dir.path().to_path_buf(),
    };

    let server = Arc::new(Server::new(config).expect("Server initialization should succeed"));

    tokio::spawn(async move {
        let _ = server.run().await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port))
        .await
        .expect("Client should connect to TCP server");

    let ping_frame = Frame::new(MessageType::Ping, 10, vec![]);
    stream.write_all(&ping_frame.encode()).await.unwrap();

    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await.unwrap();
    let (resp_frame, _) = Frame::decode(&buf[..n]).unwrap();
    assert_eq!(resp_frame.header.message_type, MessageType::Pong);
    assert_eq!(resp_frame.header.request_id, 10);

    let query_frame = MessageCodec::encode_query(11, "CREATE TABLE cars (id INTEGER, brand TEXT);");
    stream.write_all(&query_frame.encode()).await.unwrap();

    let n = stream.read(&mut buf).await.unwrap();
    let (complete_frame, _) = Frame::decode(&buf[..n]).unwrap();
    assert_eq!(
        complete_frame.header.message_type,
        MessageType::CommandComplete
    );

    let insert_frame = MessageCodec::encode_query(
        12,
        "INSERT INTO cars (id, brand) VALUES (1, 'Tesla'), (2, 'Porsche');",
    );
    stream.write_all(&insert_frame.encode()).await.unwrap();

    let n = stream.read(&mut buf).await.unwrap();
    let (insert_complete, _) = Frame::decode(&buf[..n]).unwrap();
    assert_eq!(
        insert_complete.header.message_type,
        MessageType::CommandComplete
    );

    let select_frame = MessageCodec::encode_query(13, "SELECT id, brand FROM cars WHERE id = 1;");
    stream.write_all(&select_frame.encode()).await.unwrap();

    let mut stream_buf = Vec::new();
    let mut tmp = [0u8; 4096];
    let mut frames = Vec::new();

    while frames.len() < 3 {
        let n = stream.read(&mut tmp).await.unwrap();
        if n == 0 {
            break;
        }
        stream_buf.extend_from_slice(&tmp[..n]);

        while !stream_buf.is_empty() {
            match Frame::decode(&stream_buf) {
                Ok((frame, bytes_read)) => {
                    stream_buf.drain(..bytes_read);
                    frames.push(frame);
                }
                Err(_) => break,
            }
        }
    }

    assert!(frames.len() >= 2);
    assert_eq!(frames[0].header.message_type, MessageType::RowDescription);
    assert_eq!(frames[1].header.message_type, MessageType::DataRow);
}
