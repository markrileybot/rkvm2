extern crate core;

use futures::SinkExt;
use futures::stream::StreamExt;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::Framed;
use rkvm2_config::Config;

use rkvm2_input::linux::EventManager;
use rkvm2_pipe::pipe;
use rkvm2_pipe::pipe::INPUT_PIPE_NAME;
use rkvm2_proto::{Message, MessageCodec};
use rkvm2_proto::message::Payload;

#[tokio::main]
async fn main() {
    env_logger::init();
    let config = Config::read();
    log::debug!("Awaiting connection");
    let stream = pipe::accept(INPUT_PIPE_NAME, config.socket_gid).await;
    handle_stream(stream, config.commander).await;
}

async fn handle_stream<T: AsyncRead + AsyncWrite>(stream: T, commander: bool) {
    let (mut sink, mut source) = Framed::new(stream, MessageCodec::new()).split();
    let mut event_manager = EventManager::new()
        .await
        .expect("Failed to create event manager");
    log::debug!("Received connection");

    loop {
        tokio::select! {
            event = event_manager.read() => {
                match event {
                    Ok(input_event) => {
                        if commander {
                            if let Err(e) = sink.send(Message {header: None, payload: Some(Payload::InputEvent(input_event))}).await {
                                log::warn!("Failed to send input event {}", e);
                                return;
                            }
                        } else {
                            if let Err(e) = event_manager.write(input_event).await {
                                panic!("Error sending input event {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        panic!("Error receiving input event {}", e);
                    }
                }
            }
            maybe_msg = source.next() => {
                match maybe_msg {
                    Some(Ok(Message {header: _, payload: Some(Payload::InputEvent(input_event))})) => {
                        if let Err(e) = event_manager.write(input_event).await {
                            log::warn!("Failed to write input event {:?}", e);
                        }
                    }
                    Some(Ok(Message {header: _, payload: Some(Payload::PingEvent(_))})) => {
                        // ignore
                    }
                    Some(Ok(message)) => {
                        log::warn!("Invalid message type {:?}", message);
                    }
                    Some(Err(e)) => {
                        panic!("Failed to parse message {}", e);
                    }
                    None => {
                    }
                }
            }
        }
    }
}
