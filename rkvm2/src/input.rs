use std::io;
use std::io::Error;

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use futures::stream::{SplitSink, SplitStream};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::codec::Framed;

use rkvm2_pipe::pipe;
use rkvm2_pipe::pipe::{ClientPipeStream, INPUT_PIPE_NAME};
use rkvm2_proto::{Message, MessageCodec};

use crate::conn::{Connection, Connector, MessageSink, MessageStream};

pub struct StreamSink<T: AsyncRead + AsyncWrite + Send> {
    sink: SplitSink<Framed<T, MessageCodec<Message>>, Message>,
}
#[async_trait]
impl <T: AsyncRead + AsyncWrite + Send> MessageSink for StreamSink<T> {
    async fn send(&mut self, message: Message) -> Result<(), Error> {
        self.sink.send(message).await
    }
}
pub struct StreamStream<T: AsyncRead + AsyncWrite + Send> {
    stream: SplitStream<Framed<T, MessageCodec<Message>>>,
}
#[async_trait]
impl <T: AsyncRead + AsyncWrite + Send> MessageStream for StreamStream<T> {
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
    type SinkType = StreamSink<ClientPipeStream>;
    type StreamType = StreamStream<ClientPipeStream>;
    async fn connect(&self) -> io::Result<(Self::SinkType, Self::StreamType)> {
        let stream = pipe::connect(INPUT_PIPE_NAME).await?;
        let (sink, stream) = Framed::new(stream, MessageCodec::new()).split();
        Ok((StreamSink { sink }, StreamStream { stream }))
    }
}
