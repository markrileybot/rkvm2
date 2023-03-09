extern crate core;

use std::collections::HashSet;
use std::iter::FromIterator;
use std::time::{Duration, Instant};

use arboard::Clipboard;
use itertools::Itertools;
use notify_rust::{Notification, NotificationHandle};
use num_traits::cast::ToPrimitive;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::time::sleep;

use rkvm2_config::Config;
use rkvm2_proto::{ActiveNodeChangedEvent, ClipboardEvent, Header, InputEvent, Key, KeyEvent, Message, PingEvent};
use rkvm2_proto::input_event::InputEventType;
use rkvm2_proto::message::Payload;

use crate::input::InputClient;
use crate::net::Distributor;

mod conn;
mod input;
mod net;

trait Action: Send {
    fn act(&self, app: &App);
}

struct ActiveNodeChangeAction {
    node_index: Option<usize>
}
impl ActiveNodeChangeAction {
    fn for_next_node() -> Self {
        Self {
            node_index: None,
        }
    }
    fn for_node(node_index: usize) -> Self {
        Self {
            node_index: Some(node_index),
        }
    }
}
impl Action for ActiveNodeChangeAction {
    fn act(&self, app: &App) {
        let next_node_index = self.node_index.unwrap_or((app.active_node + 1) % app.nodes.len());
        if let Some(next_node) = app.nodes.get(next_node_index) {
            let name = next_node.name.clone();
            app.send_to_loopback(Message {
                header: None,
                payload: Some(Payload::ActiveNodeChangedEvent(ActiveNodeChangedEvent {
                    name,
                })),
            });
        }
    }
}

struct KeyBinding {
    keys: HashSet<i32>,
    action: Box<dyn Action>,
}
impl KeyBinding {
    fn new(keys: Vec<Key>, action: Box<dyn Action>) -> Self {
        Self {
            keys: HashSet::from_iter(keys.iter().map(|k| k.to_i32().unwrap())),
            action
        }
    }
    fn act(&self, app: &App) {
        if self.keys == app.keys {
            self.action.act(app);
        }
    }
}

#[derive(Debug)]
struct Node {
    commander: bool,
    local: bool,
    name: String,
    last_heard_from: Instant,
}
impl Node {
    fn expired(&self, now: Instant) -> bool {
        return !self.commander
            && !self.local
            && now.duration_since(self.last_heard_from) > Duration::from_secs(3);
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
    current_notification: Option<NotificationHandle>
}

impl App {
    async fn run(name: String, config: Config) {
        let (message_sender, mut message_receiver) = unbounded_channel();
        let input_sender = InputClient::open(message_sender.clone());
        let net_sender = Distributor::open(config.broadcast_address, message_sender.clone());
        let ping_sender = message_sender.clone();

        let my_node = Node {
            commander: config.commander,
            local: true,
            name,
            last_heard_from: Instant::now(),
        };

        let key_bindings = vec![
            KeyBinding::new(config.switch_keys, Box::new(ActiveNodeChangeAction::for_next_node())),
            KeyBinding::new(config.commander_keys, Box::new(ActiveNodeChangeAction::for_node(0))),
        ];

        let mut app = Self {
            nodes: vec![my_node],
            keys: Default::default(),
            active_node: if config.commander {0} else {usize::MAX},
            key_bindings,
            input_sender,
            net_sender,
            message_sender,
            current_notification: None,
        };

        tokio::spawn(async move {
            loop {
                if let Err(e) = ping_sender.send(Message {
                    header: None,
                    payload: Some(Payload::PingEvent(PingEvent::default())),
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
            from_id: my_node.name.clone(),
            to_id: to_id.to_string(),
            ..Header::default()
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

    fn handle_message(&mut self, message: Message) {
        log::trace!("{:?}", message);
        let mut origin = String::new();
        let mut from_net = false;

        if let Some(header) = &message.header {
            let my_node = self.nodes.get(0).unwrap();
            if header.from_id == my_node.name {
                // external messages that are from me
                return;
            } else if !header.to_id.is_empty() && header.to_id != my_node.name {
                // external messages that aren't for me
                return;
            } else {
                // external messages that aren't for me
                origin = header.from_id.clone();
            }
            from_net = !origin.is_empty();
        }

        if let Some(payload) = &message.payload {
            match payload {
                Payload::PingEvent(ping) => {
                    self.handle_ping(from_net, origin, ping);
                }
                Payload::InputEvent(_) => {
                    self.handle_input(message);
                }
                Payload::ActiveNodeChangedEvent(active_node_changed) => {
                    self.handle_active_node_changed(from_net, active_node_changed)
                }
                Payload::ClipboardEvent(clipboard) => {
                    self.handle_clipboard(clipboard);
                }
                _ => {
                    if !from_net {
                        self.send_to_net(message, "");
                    }
                }
            }
        }
    }

    fn handle_active_node_changed(&mut self, from_net: bool, active_node_changed: &ActiveNodeChangedEvent) {
        if let Some((new_active_node, node)) =
            self.nodes.iter().find_position(|n| n.name == active_node_changed.name)
        {
            if self.active_node != new_active_node {
                // my node is active
                if self.active_node == 0 {
                    // if I'm about to be switched, send my clip contents
                    match Clipboard::new() {
                        Ok(mut clipboard) => {
                            match clipboard.get_text() {
                                Ok(text) => {
                                    log::debug!("Send clip text\n{}", text);
                                    self.send_to_net(
                                        Message {
                                            header: None,
                                            payload: Some(Payload::ClipboardEvent(ClipboardEvent {
                                                data: text.into_bytes(),
                                                mime_type: "".to_string(),
                                            })),
                                        },
                                        "",
                                    )
                                }
                                Err(e) => {
                                    log::warn!("Failed to get clipboard text {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to get clipboard {}", e);
                        }
                    }

                    // release any keybinding keys
                    for key in &self.keys {
                        self.send_to_input(Message {
                            header: None,
                            payload: Some(Payload::InputEvent(InputEvent {
                                input_event_type: Some(InputEventType::Key(KeyEvent {
                                    key: key.clone(),
                                    down: false,
                                })),
                            })),
                        });
                    }
                    self.keys.clear();
                }

                // switch the active node
                self.active_node = new_active_node;
                log::debug!("Switched to {:?}", node);

                let active_node_name = node.name.clone();
                if new_active_node == 0 {
                    self.notify("I'm over here!");
                } else {
                    self.notify(format!("Switched to {}", active_node_name).as_str());
                }

                if !from_net {
                    self.send_to_net(Message {
                        header: None,
                        payload: Some(Payload::ActiveNodeChangedEvent(ActiveNodeChangedEvent {
                            name: active_node_name,
                        })),
                    }, "");
                }
            }
        } else {
            log::debug!("New active node {} not found", active_node_changed.name);
        }
    }

    fn handle_input(&mut self, message: Message) {
        // track the keys.  Any keys remaining after a switch should be released
        let keys_changed = match &message {
            Message { header: _, payload: Some(Payload::InputEvent(InputEvent { input_event_type: Some(InputEventType::Key(key_event)) })) } => {
                match key_event.down {
                    true => self.keys.insert(key_event.key),
                    false => self.keys.remove(&key_event.key),
                }
            }
            _ => false,
        };

        if keys_changed {
            let my_node = self.nodes.get(0).unwrap();
            if my_node.commander {
                for key_binding in &self.key_bindings {
                    key_binding.act(self);
                }
            }
        }

        if let Some(active_node) = self.nodes.get(self.active_node) {
            if active_node.local {
                self.send_to_input(message);
                return;
            }

            let my_node = self.nodes.get(0).unwrap();
            if my_node.commander {
                self.send_to_net(message, active_node.name.as_str())
            }
        } else {
            // we couldn't find the active node.  Could have expired and we haven't switched
            // back to the commander yet.
            self.send_to_input(message);
            return;
        }
    }

    fn handle_ping(&mut self, from_net: bool, origin: String, ping: &PingEvent) {
        if from_net {
            if let Some(node) =
                self.nodes.iter_mut().find(|n| n.name == origin)
            {
                node.last_heard_from = Instant::now();
                node.commander = ping.commander;
            } else {
                log::info!("Adding {}", origin);
                self.nodes.push(Node {
                    commander: ping.commander,
                    local: false,
                    name: origin.clone(),
                    last_heard_from: Instant::now(),
                });
            }

            // if we got the ping from the commander, make sure we're tracking state properly
            if ping.commander {
                if let Some((pos, _)) = self.nodes.iter().find_position(|n| n.name == ping.active_node) {
                    if self.active_node == pos {
                        // we don't need the extra event
                        return;
                    }
                }

                // send an event on loopback that looks like an active node changed event from the commander
                self.send_to_loopback(Message {
                    header: Some(Header {
                        from_id: origin,
                        ..Header::default()
                    }),
                    payload: Some(Payload::ActiveNodeChangedEvent(ActiveNodeChangedEvent {
                        name: ping.active_node.clone(),
                    })),
                });
            }
        } else {
            let now = Instant::now();
            for (index, node) in self.nodes.iter().enumerate() {
                if node.expired(now) {
                    log::info!("Expiring {}", node.name);

                    if self.active_node == index {
                        if let Some(commander_name) = self.nodes.iter()
                            .find(|n| n.commander)
                            .map(|n| n.name.clone()) {

                            self.send_to_loopback(Message {
                                header: None,
                                payload: Some(Payload::ActiveNodeChangedEvent(ActiveNodeChangedEvent {
                                    name: commander_name,
                                })),
                            });
                        }
                    }
                }
            }
            self.nodes.retain(|n| !n.expired(now));

            let my_node = self.nodes.get(0).unwrap();
            self.send_to_net(Message {
                header: None,
                payload: Some(Payload::PingEvent(PingEvent {
                    commander: my_node.commander,
                    active_node: if my_node.commander {
                        if let Some(n) = self.nodes.get(self.active_node) {
                            n.name.clone()
                        } else {
                            "".to_string()
                        }
                    } else {
                        "".to_string()
                    },
                })),
            }, "");
        }
    }

    fn handle_clipboard(&self, clipboard: &ClipboardEvent) {
        let text = String::from_utf8_lossy(&clipboard.data);
        log::debug!("Got clip text\n{}", text);
        match Clipboard::new() {
            Ok(mut clipboard) => {
                if let Err(e) = clipboard.set_text(text) {
                    log::warn!("Failed to set clipboard text {}", e);
                }
            }
            Err(e) => {
                log::warn!("Failed to get clipboard {}", e);
            }
        }
    }

    fn notify(&mut self, message: &str)  {
        match Notification::new()
            .summary("RKVM")
            .body(message)
            .show() {
            Ok(notification_handle) => {
                if let Some(previous_handle) = self.current_notification.replace(notification_handle) {
                    previous_handle.close();
                }
            }
            Err(e) => {
                log::debug!("Failed to notify {}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let config = Config::read();
    App::run(hostname::get().unwrap().into_string().unwrap(), config).await;
}
