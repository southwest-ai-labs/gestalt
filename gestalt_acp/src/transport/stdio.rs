use std::future::Future;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};

pub struct StdioTransport<R, W> {
    reader: BufReader<R>,
    writer: W,
}

impl<R, W> StdioTransport<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: BufReader::new(reader),
            writer,
        }
    }

    pub async fn run_loop<H, F>(&mut self, mut handler: H) -> std::io::Result<()>
    where
        H: FnMut(serde_json::Value) -> F,
        F: Future<Output = Option<serde_json::Value>>,
    {
        let mut line = String::new();
        loop {
            line.clear();
            let bytes_read = self.reader.read_line(&mut line).await?;
            if bytes_read == 0 {
                // EOF
                break;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            match serde_json::from_str::<serde_json::Value>(trimmed) {
                Ok(req) => {
                    if let Some(res) = handler(req).await {
                        let mut res_bytes = serde_json::to_vec(&res).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                        res_bytes.push(b'\n');
                        self.writer.write_all(&res_bytes).await?;
                        self.writer.flush().await?;
                    }
                }
                Err(_) => {
                    let err_res = serde_json::json!({
                        "jsonrpc": "2.0",
                        "error": {
                            "code": -32700,
                            "message": "Parse error"
                        },
                        "id": serde_json::Value::Null
                    });
                    let mut res_bytes = serde_json::to_vec(&err_res).unwrap();
                    res_bytes.push(b'\n');
                    self.writer.write_all(&res_bytes).await?;
                    self.writer.flush().await?;
                }
            }
        }
        Ok(())
    }
}
