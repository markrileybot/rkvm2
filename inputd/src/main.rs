use std::fs;
use std::io::ErrorKind::AddrInUse;
use futures::SinkExt;
use futures::stream::StreamExt;
use tokio::net::{UnixListener, UnixStream};
use tokio_util::codec::Framed;

use rkvm2_input::linux::EventManager;
use rkvm2_proto::{Message, MessageCodec};
use rkvm2_proto::message::Payload;

#[tokio::main]
async fn main() {
    env_logger::init();
    loop {
        log::info!("Bind...");
        match UnixListener::bind("/var/run/rkvm2.sock") {
            Ok(listener) => {
                match listener.accept().await {
                    Ok((stream, _addr)) => {
                        handle_stream(stream).await;
                    }
                    Err(e) => {
                        log::warn!("Accept failed {}", e);
                    }
                }
            }
            Err(e) => {
                if e.kind() == AddrInUse {
                    fs::remove_file("/var/run/rkvm2.sock").expect(format!("Failed to remove existing socket {}", e).as_str());
                } else {
                    panic!("Failed to bind to socket {}", e);
                }
            }
        };
    }
}

async fn handle_stream(stream: UnixStream) {
    // here we should await authentication

    let mut event_manager = EventManager::new().await.expect("Failed to create event manager");
    let (mut sink, mut source) = Framed::new(stream, MessageCodec::new()).split();
    log::info!("Opened event manager");

    loop {
        tokio::select! {
            event = event_manager.read() => {
                match event {
                    Ok(input_event) => {
                        log::info!("Send {:?}", input_event);
                        let _ = sink.send(Message {
                            header: None,
                            payload: Some(Payload::InputEvent(input_event.clone()))
                        }).await;
                        let _ = event_manager.write(input_event).await;
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