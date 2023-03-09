use std::io;
use std::io::Error;
use std::net::SocketAddr;
use std::str::FromStr;

use async_trait::async_trait;
use futures::stream::StreamExt;
use futures::stream::{SplitSink, SplitStream};
use futures::SinkExt;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::udp::UdpFramed;

use rkvm2_proto::{Message, MessageCodec};

use crate::conn::{Connection, Connector, MessageSink, MessageStream};

pub struct UdpSink {
    sink: SplitSink<UdpFramed<MessageCodec<Message>>, (Message, SocketAddr)>,
    socket_address: SocketAddr,
}
#[async_trait]
impl MessageSink for UdpSink {
    async fn send(&mut self, message: Message) -> Result<(), io::Error> {
        self.sink.send((message, self.socket_address)).await
    }
}
pub struct UdpStream {
    stream: SplitStream<UdpFramed<MessageCodec<Message>>>,
}
#[async_trait]
impl MessageStream for UdpStream {
    async fn next(&mut self) -> Option<Result<Message, Error>> {
        match self.stream.next().await {
            None => None,
            Some(Ok((message, _))) => Some(Ok(message)),
            Some(Err(e)) => Some(Err(e)),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Distributor {
    broadcast_address: String,
}
impl Distributor {
    pub(crate) fn open(
        broadcast_address: String,
        sender: UnboundedSender<Message>,
    ) -> UnboundedSender<Message> {
        Connection::open(Self { broadcast_address }, sender)
    }
}

#[async_trait]
impl Connector for Distributor {
    type SinkType = UdpSink;
    type StreamType = UdpStream;
    async fn connect(&self) -> io::Result<(Self::SinkType, Self::StreamType)> {
        log::info!("Connect to {}", self.broadcast_address);
        let socket_address = SocketAddr::from_str(self.broadcast_address.as_str()).unwrap();
        let socket = UdpSocket::bind(socket_address).await?;
        socket.set_broadcast(true)?;
        let (sink, stream) = UdpFramed::new(socket, MessageCodec::new()).split();
        return Ok((
            UdpSink {
                sink,
                socket_address,
            },
            UdpStream { stream },
        ));
    }
}
