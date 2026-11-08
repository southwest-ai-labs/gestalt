#[path = "../src/observe/stream_interceptor.rs"]
mod stream_interceptor;

use stream_interceptor::{StreamChunk, StreamInterceptor};
use tokio::io::duplex;
use tokio::io::AsyncWriteExt;

#[tokio::test]
async fn test_stream_interceptor_single_stream() {
    let (mut writer, reader) = duplex(1024);

    let handle = tokio::spawn(async move {
        writer.write_all(b"chunk 1\nchunk 2\nchunk 3\n").await.unwrap();
        drop(writer);
    });

    let mut collected = Vec::new();
    StreamInterceptor::pipe_stream("stdout", "agent-1", "run-123", reader, |chunk: StreamChunk| {
        collected.push(chunk);
    })
    .await
    .unwrap();

    handle.await.unwrap();

    assert_eq!(collected.len(), 3);
    assert_eq!(collected[0].chunk, "chunk 1");
    assert_eq!(collected[0].stream, "stdout");
    assert_eq!(collected[0].agent_id, "agent-1");
    assert_eq!(collected[0].run_id, "run-123");
    assert_eq!(collected[1].chunk, "chunk 2");
    assert_eq!(collected[2].chunk, "chunk 3");
}

#[tokio::test]
async fn test_stream_interceptor_concurrent_channel() {
    let (mut out_w, out_r) = duplex(1024);
    let (mut err_w, err_r) = duplex(1024);

    let (tx, mut rx) = tokio::sync::mpsc::channel(100);

    let writer_handle = tokio::spawn(async move {
        out_w.write_all(b"stdout line 1\nstdout line 2\n").await.unwrap();
        err_w.write_all(b"stderr line 1\n").await.unwrap();
        drop(out_w);
        drop(err_w);
    });

    StreamInterceptor::pipe_to_channel("agent-2", "run-456", Some(out_r), Some(err_r), tx).await;
    writer_handle.await.unwrap();

    let mut chunks = Vec::new();
    while let Ok(chunk) = rx.try_recv() {
        chunks.push(chunk);
    }

    assert_eq!(chunks.len(), 3);
    assert!(chunks.iter().any(|c| c.stream == "stdout" && c.chunk == "stdout line 1"));
    assert!(chunks.iter().any(|c| c.stream == "stdout" && c.chunk == "stdout line 2"));
    assert!(chunks.iter().any(|c| c.stream == "stderr" && c.chunk == "stderr line 1"));
}
