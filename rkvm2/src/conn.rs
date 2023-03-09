use std::fmt::Debug;
use std::io;
use std::io::Error;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::time::sleep;

use rkvm2_proto::Message;

#[async_trait]
pub trait MessageSink: Send {
    // send that frame
    async fn send(&mut self, message: Message) -> Result<(), Error>;
}

#[async_trait]
pub trait MessageStream: Send {
    // get that frame
    async fn next(&mut self) -> Option<Result<Message, Error>>;
}

#[async_trait]
pub trait Connector: Send + Sync + Debug {
    /// The type of thing this returns
    type SinkType: MessageSink;
    /// The type of thing this returns
    type StreamType: MessageStream;

    /// split the connection bro
    async fn connect(&self) -> io::Result<(Self::SinkType, Self::StreamType)>;
}

pub(crate) struct Connection;
impl Connection {
    pub(crate) fn open<T: Connector + 'static>(
        connector: T,
        sender: UnboundedSender<Message>,
    ) -> UnboundedSender<Message> {
        let (ret_sender, mut receiver) =
            unbounded_channel() as (UnboundedSender<Message>, UnboundedReceiver<Message>);

        tokio::spawn(async move {
            loop {
                match connector.connect().await {
                    Ok((mut sink, mut stream)) => loop {
                        tokio::select! {
                            maybe_msg = stream.next() => {
                                match maybe_msg {
                                    Some(Ok(message)) => {
                                        let _ = sender.send(message);
                                    }
                                    Some(Err(e)) => {
                                        log::warn!("Failed to read message {}", e);
                                        break;
                                    }
                                    None => {}
                                }
                            }
                            maybe_msg = receiver.recv() => {
                                if let Some(message) = maybe_msg {
                                    if let Err(e) = sink.send(message).await {
                                        log::warn!("Failed to send {}", e);
                                        break;
                                    }
                                }
                            }
                        }
                    },
                    Err(e) => {
                        log::warn!("Failed to open {:?}. {}", connector, e);
                        sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        });

        return ret_sender;
    }
}
