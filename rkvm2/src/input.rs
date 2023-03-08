use std::io;
use std::io::Error;

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use futures::stream::{SplitSink, SplitStream};
use tokio::net::UnixStream;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::codec::Framed;

use rkvm2_proto::{Message, MessageCodec};

use crate::conn::{Connection, Connector, MessageSink, MessageStream};

pub struct UnixStreamSink {
    sink: SplitSink<Framed<UnixStream, MessageCodec<Message>>, Message>
}
#[async_trait]
impl MessageSink for UnixStreamSink {
    async fn send(&mut self, message: Message) -> Result<(), Error> {
        self.sink.send(message).await
    }
}
pub struct UnixStreamStream {
    stream: SplitStream<Framed<UnixStream, MessageCodec<Message>>>
}
#[async_trait]
impl MessageStream for UnixStreamStream {
    async fn next(&mut self) -> Option<Result<Message, Error>> {
        self.stream.next().await
    }
}

#[derive(Debug)]
pub(crate) struct InputClient;
impl InputClient {
    pub(crate) fn open(sender: UnboundedSender<Message>) -> UnboundedSender<Message> {
        Connection::open(Self {}, sender)
    }
}
#[async_trait]
impl Connector for InputClient {
    type SinkType = UnixStreamSink;
    type StreamType = UnixStreamStream;
    async fn connect(&self) -> io::Result<(Self::SinkType, Self::StreamType)> {
        log::info!("Open /var/run/rkvm2.sock");
        let stream = UnixStream::connect("/var/run/rkvm2.sock").await?;
        let (sink, stream) = Framed::new(stream, MessageCodec::new()).split();
        Ok((UnixStreamSink {sink}, UnixStreamStream {stream}))
    }
}