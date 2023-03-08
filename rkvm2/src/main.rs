extern crate core;

use std::collections::HashSet;
use std::time::{Duration, Instant};

use itertools::Itertools;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::time::sleep;

use rkvm2_proto::{ActiveNodeChangedEvent, ClipboardEvent, Header, Message, PingEvent};
use rkvm2_proto::input_event::InputEventType;
use rkvm2_proto::message::Payload;

use crate::net::Distributor;
use crate::input::InputClient;

mod input;
mod net;
mod conn;

trait Action: Send {
    fn act(&self, app: &App);
}

struct ActiveNodeChangeAction;
impl Action for ActiveNodeChangeAction {
    fn act(&self, app: &App) {
        if let Some(next_node) = app.nodes.get((app.active_node + 1) % app.nodes.len()) {
            let name = next_node.name.clone();
            app.send_to_loopback(Message {
                header: None,
                payload: Some(Payload::ActiveNodeChangedEvent(ActiveNodeChangedEvent {name})),
            });
        }
    }
}

struct KeyBinding {
    keys: HashSet<i32>,
    action: Box<dyn Action>,
}
impl KeyBinding {
    fn act(&self, app: &App) {
        if self.keys == app.keys {
            self.action.act(app)
        }
    }
}

struct Node {
    commander: bool,
    local: bool,
    name: String,
    last_heard_from: Instant
}
impl Node {
    fn expired(&self, now: Instant) -> bool {
        return !self.commander && !self.local && now.duration_since(self.last_heard_from) > Duration::from_secs(3);
    }
}

struct App {
    nodes: Vec<Node>,
    keys: HashSet<i32>,
    active_node: usize,
    key_bindings: Vec<KeyBinding>,
    input_sender: UnboundedSender<Message>,
    net_sender: UnboundedSender<Message>,
    message_sender: UnboundedSender<Message>,
}

impl App {
    async fn run(name: String, broadcast_address: String) {
        let (message_sender, mut message_receiver) = unbounded_channel();
        let input_sender = InputClient::open(message_sender.clone());
        let net_sender  = Distributor::open(broadcast_address, message_sender.clone());
        let ping_sender = message_sender.clone();

        let my_node = Node {
            commander: false,
            local: true,
            name,
            last_heard_from: Instant::now()
        };

        let mut app = Self {
            nodes: vec![my_node],
            keys: Default::default(),
            active_node: 0,
            key_bindings: Default::default(),
            input_sender,
            net_sender,
            message_sender
        };

        tokio::spawn(async move {
            loop {
                if let Err(e) = ping_sender.send(Message {
                    header: None,
                    payload: Some(Payload::PingEvent(PingEvent { commander: false })),
                }) {
                    log::warn!("Failed to send ping {}", e);
                }
                sleep(Duration::from_secs(1)).await;
            }
        });

        loop {
            if let Some(message) = message_receiver.recv().await {
                app.handle_message(message)
            }
        }
    }

    fn send_to_input(&self, message: Message) {
        if let Err(e) = self.input_sender.send(message) {
            log::warn!("Failed to send message {}", e);
        }
    }

    fn send_to_net(&self, mut message: Message, to_id: &str) {
        let my_node = self.nodes.get(0).unwrap();
        message.header = Some(Header {
            id: "".to_string(),
            from_id: my_node.name.clone(),
            to_id: to_id.to_string(),
            time: None,
            header_type: None,
        });
        if let Err(e) = self.net_sender.send(message) {
            log::warn!("Failed to send message {}", e);
        }
    }

    fn send_to_loopback(&self, message: Message) {
        if let Err(e) = self.message_sender.send(message) {
            log::warn!("Failed to send message {}", e);
        }
    }

    fn handle_message(&mut self, mut message: Message) {
        let mut send_out = false;
        let mut send_destination = "";
        let mut recv_source = "";

        match &message {
            Message { header: maybe_header, payload: Some(payload) } => {
                match maybe_header {
                    None => {
                        // internal messages that want to be sent out have no header
                        send_out = true;
                    }
                    Some(header) => {
                        let my_node = self.nodes.get(0).unwrap();

                        // external messages that are from me
                        if header.from_id == my_node.name {
                            return;
                        }
                        // external messages that aren't for me
                        if !header.to_id.is_empty() && header.to_id != my_node.name  {
                            return;
                        }

                        recv_source = header.from_id.as_str();
                    }
                }

                match payload {
                    Payload::Empty(_) => {}
                    Payload::NotifyEvent(_) => {}
                    Payload::PingEvent(ping) => {
                        if send_out {
                            let my_node = self.nodes.get(0).unwrap();
                            message = Message {
                                header: None,
                                payload: Some(Payload::PingEvent(PingEvent { commander: my_node.commander })),
                            };

                            let now = Instant::now();
                            self.nodes.retain(|n| {
                                if n.expired(now) {
                                    log::info!("Expiring {}", n.name);
                                    return false;
                                }
                                return true;
                            });
                        } else {
                            if let Some(node) = self.nodes.iter_mut().find(|n| n.name == recv_source) {
                                node.last_heard_from = Instant::now();
                                node.commander = ping.commander;
                            } else {
                                self.nodes.push(Node {
                                    commander: ping.commander,
                                    local: false,
                                    name: recv_source.to_string(),
                                    last_heard_from: Instant::now(),
                                });
                            }
                            return;
                        }
                    }
                    Payload::ClipboardEvent(_) => {

                    }
                    Payload::InputEvent(e) => {
                        let my_node = self.nodes.get_mut(0).unwrap();

                        // if we're getting an input event without a header that means we're the commander
                        my_node.commander = send_out;

                        if my_node.commander {
                            if let Some(InputEventType::Key(key_event)) = &e.input_event_type {
                                let changed = match key_event.down {
                                    true => self.keys.insert(key_event.key),
                                    false => self.keys.remove(&key_event.key)
                                };

                                if changed {
                                    for key_binding in &self.key_bindings {
                                        key_binding.act(self);
                                    }
                                }
                            }
                        }

                        let active_node = self.nodes.get(self.active_node).unwrap();
                        send_destination = active_node.name.as_str();
                        if active_node.local {
                            self.send_to_input(message);
                            return;
                        }
                    }
                    Payload::ActiveNodeChangedEvent(e) => {
                        let my_node = self.nodes.get(0).unwrap();
                        let active_node = self.nodes.get(self.active_node).unwrap();

                        if active_node.name == my_node.name {
                            // if I'm about to be switched, send my clip contents
                            self.send_to_net(Message {
                                header: None,
                                payload: Some(Payload::ClipboardEvent(ClipboardEvent { data: vec![], mime_type: "".to_string() })),
                            }, "")
                        }
                        if let Some((new_active_node, _)) = self.nodes.iter().find_position(|n| n.name == e.name) {
                            // switch the active node
                            self.active_node = new_active_node;
                        }
                    }
                }
            }
            _ => {}
        }

        if send_out {
            self.send_to_net(message, send_destination);
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    App::run(hostname::get().unwrap().into_string().unwrap(), "192.168.24.255:54361".to_string()).await;
}