use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct StreamChunk {
    pub stream: String,
    pub chunk: String,
    pub agent_id: String,
    pub run_id: String,
}

pub struct StreamInterceptor;

impl StreamInterceptor {
    /// Pipes an AsyncRead stream (such as ChildStdout or ChildStderr) line by line,
    /// invoking the callback for each chunk in real time.
    pub async fn pipe_stream<R, F>(
        stream_name: &str,
        agent_id: &str,
        run_id: &str,
        reader: R,
        mut on_chunk: F,
    ) -> std::io::Result<()>
    where
        R: AsyncRead + Unpin,
        F: FnMut(StreamChunk) + Send,
    {
        let mut lines = BufReader::new(reader).lines();
        while let Some(line) = lines.next_line().await? {
            let chunk = StreamChunk {
                stream: stream_name.to_string(),
                chunk: line,
                agent_id: agent_id.to_string(),
                run_id: run_id.to_string(),
            };
            on_chunk(chunk);
        }
        Ok(())
    }

    /// Pipes both stdout and stderr concurrently, broadcasting chunks to a shared sender.
    pub async fn pipe_to_channel<R1, R2>(
        agent_id: &str,
        run_id: &str,
        stdout: Option<R1>,
        stderr: Option<R2>,
        tx: tokio::sync::mpsc::Sender<StreamChunk>,
    ) where
        R1: AsyncRead + Unpin + Send + 'static,
        R2: AsyncRead + Unpin + Send + 'static,
    {
        let mut tasks = Vec::new();

        if let Some(out) = stdout {
            let tx_out = tx.clone();
            let aid = agent_id.to_string();
            let rid = run_id.to_string();
            tasks.push(tokio::spawn(async move {
                let _ = Self::pipe_stream("stdout", &aid, &rid, out, move |c| {
                    let _ = tx_out.try_send(c);
                }).await;
            }));
        }

        if let Some(err) = stderr {
            let tx_err = tx.clone();
            let aid = agent_id.to_string();
            let rid = run_id.to_string();
            tasks.push(tokio::spawn(async move {
                let _ = Self::pipe_stream("stderr", &aid, &rid, err, move |c| {
                    let _ = tx_err.try_send(c);
                }).await;
            }));
        }

        for task in tasks {
            let _ = task.await;
        }
    }
}
