//! The same wire types as Godot and core.wasm; socket IO never blocks the UI.
use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use openheart_core::protocol::{self, Hello, ServerMessage};
use std::{sync::mpsc, thread, time::Duration};
use tokio::sync::mpsc as channel;
use tokio_tungstenite::tungstenite::{protocol::WebSocketConfig, Message};

pub struct Network {
    tx: Option<channel::Sender<String>>,
    pub rx: mpsc::Receiver<Result<ServerMessage, String>>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Network {
    pub fn connect(url: String, hello: Hello) -> Result<Self> {
        anyhow::ensure!(
            url.starts_with("ws://") || url.starts_with("wss://"),
            "Адрес должен начинаться с ws:// или wss://"
        );
        let (tx, mut outgoing) = channel::channel::<String>(128);
        let (incoming, rx) = mpsc::sync_channel(256);
        let worker = thread::spawn(move || {
            let result: Result<()> = (|| {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                runtime.block_on(async {
                    let config = WebSocketConfig::default().max_message_size(Some(1024 * 1024));
                    let (mut socket, _) = tokio::time::timeout(Duration::from_secs(10),
                        tokio_tungstenite::connect_async_with_config(url, Some(config), false)).await??;
                    socket.send(Message::Text(protocol::encode("hello", &hello)?.into())).await?;
                    let mut last_received = tokio::time::Instant::now();
                    let mut watchdog = tokio::time::interval(Duration::from_secs(1));
                    loop {
                        tokio::select! {
                            _ = watchdog.tick() => {
                                anyhow::ensure!(last_received.elapsed() < Duration::from_secs(15), "Сервер перестал отвечать");
                            }
                            raw = outgoing.recv() => match raw {
                                Some(raw) => tokio::time::timeout(Duration::from_secs(5), socket.send(Message::Text(raw.into()))).await??,
                                None => { let _ = tokio::time::timeout(Duration::from_secs(1), socket.close(None)).await; return Ok(()); }
                            },
                            message = socket.next() => {
                                last_received = tokio::time::Instant::now();
                                match message {
                                    Some(Ok(Message::Text(text))) => {
                                        let parsed = protocol::parse_server(&text).map_err(|e| anyhow!("Некорректное сообщение сервера: {e}"))?;
                                        incoming.try_send(Ok(parsed)).map_err(|_| anyhow!("Переполнена очередь сообщений"))?;
                                    }
                                    Some(Ok(Message::Close(reason))) => return Err(anyhow!("Сервер закрыл соединение: {reason:?}")),
                                    Some(Ok(Message::Ping(bytes))) => socket.send(Message::Pong(bytes)).await?,
                                    Some(Ok(_)) => {},
                                    Some(Err(e)) => return Err(e.into()),
                                    None => return Err(anyhow!("Соединение потеряно")),
                                }
                            }
                        }
                    }
                })
            })();
            if let Err(error) = result {
                let _ = incoming.try_send(Err(format!("{error:#}")));
            }
        });
        Ok(Self {
            tx: Some(tx),
            rx,
            worker: Some(worker),
        })
    }

    pub fn raw(&self, raw: String) -> Result<()> {
        self.tx
            .as_ref()
            .ok_or_else(|| anyhow!("Соединение закрыто"))?
            .try_send(raw)
            .map_err(|e| anyhow!("Отправка: {e}"))
    }
}

impl Drop for Network {
    fn drop(&mut self) {
        self.tx.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
