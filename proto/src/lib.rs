#[macro_use]
extern crate num_derive;
extern crate prost;
extern crate prost_wkt;
extern crate prost_wkt_types;

use std::io;
use std::marker::PhantomData;

use prost::bytes::BytesMut;
use prost::{DecodeError, Message as ProstMessage};
use tokio_util::codec::{Decoder, Encoder};

pub const RKVM2_PROTO_VERSION_STRING: &str = env!("RKVM2_PROTO_VERSION_STRING");

include!(concat!(env!("OUT_DIR"), "/rkvm2.proto.rs"));

#[derive(Default)]
pub struct MessageCodec<T: ProstMessage> {
    _marker: PhantomData<T>
}

impl MessageCodec<Message> {
    pub fn new() -> Self {
        MessageCodec::default() as MessageCodec<Message>
    }
}

impl <T: ProstMessage + Default> Decoder for MessageCodec<T> {
    type Item = T;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let message: Result<T, DecodeError> = ProstMessage::decode_length_delimited(src);
        return match message {
            Ok(t) => Ok(Some(t)),
            Err(e) => Ok(None)
        };
    }
}

impl <T: ProstMessage> Encoder<T> for MessageCodec<T> {
    type Error = io::Error;

    fn encode(&mut self, item: T, dst: &mut BytesMut) -> Result<(), Self::Error> {
        item.encode_length_delimited(dst)?;
        return Ok(());
    }
}
//
// pub trait ProtoBuilder: Sized {
//     fn with_payload(self, payload_type: PayloadType) -> Self;
//     fn with_payload_option(self, payload_type: Option<PayloadType>) -> Self {
//         match payload_type {
//             None => self,
//             Some(payload) => self.with_payload(payload)
//         }
//     }
//     fn build(self) -> Message;
//     fn build_message(header: Header, payload: Option<PayloadType>) -> Message {
//         Message {
//             header: Some(header),
//             payload: match payload {
//                 Some(p) => Some(Payload {
//                     payload_type: Some(p)
//                 }),
//                 None => None
//             }
//         }
//     }
// }
//
// pub struct EventBuilder {
//     header: Header,
//     payload_type: PayloadType
// }
// impl ProtoBuilder for EventBuilder {
//     fn with_payload(mut self, payload_type: PayloadType) -> Self {
//         self.payload_type = payload_type;
//         return self;
//     }
//     fn build(self) -> Message {
//         Self::build_message(self.header, Some(self.payload_type))
//     }
// }
//
// pub struct RequestBuilder {
//     header: Header,
//     payload_type: PayloadType
// }
// impl ProtoBuilder for RequestBuilder {
//     fn with_payload(mut self, payload_type: PayloadType) -> Self {
//         self.payload_type = payload_type;
//         return self;
//     }
//     fn build(self) -> Message {
//         Self::build_message(self.header, Some(self.payload_type))
//     }
// }
//
// pub struct ResponseBuilder {
//     header: Header,
//     payload_type: Option<PayloadType>
// }
// impl ResponseBuilder {
//     pub fn for_request(self, request_header: &Header) -> Self {
//         self.with_destination(request_header.from_id.clone(), request_header.id.clone())
//     }
//     pub fn with_result(self, result: Result<(), String>) -> Self {
//         match result {
//             Ok(_) => {
//                 self.with_code(ResponseCode::Ok)
//             }
//             Err(message) => {
//                 self.with_code(ResponseCode::Error)
//                     .with_message(message)
//             }
//         }
//     }
//     pub fn with_destination(mut self, to_id: String, request_id: String) -> Self {
//         self.header.to_id = to_id;
//         match self.header.header_type.as_mut().unwrap() {
//             HeaderType::Response(r) => {
//                 r.request_id = request_id;
//             }
//             _ => {}
//         }
//         return self;
//     }
//     pub fn with_code(mut self, code: ResponseCode) -> Self {
//         match self.header.header_type.as_mut().unwrap() {
//             HeaderType::Response(r) => {
//                 r.code = code as i32;
//             }
//             _ => {}
//         }
//         return self;
//     }
//     pub fn with_message(mut self, message: String) -> Self {
//         match self.header.header_type.as_mut().unwrap() {
//             HeaderType::Response(r) => {
//                 r.message = message;
//             }
//             _ => {}
//         }
//         return self;
//     }
// }
// impl ProtoBuilder for ResponseBuilder {
//     fn with_payload(self, payload_type: PayloadType) -> Self {
//         self.with_payload_option(Some(payload_type))
//     }
//     fn with_payload_option(mut self, payload_type: Option<PayloadType>) -> Self {
//         self.payload_type = payload_type;
//         return self;
//     }
//     fn build(self) -> Message {
//         Self::build_message(self.header, self.payload_type)
//     }
// }
//
// #[derive(Clone)]
// pub struct MessageBuilder {
//     pub client_id: String,
// }
//
// impl MessageBuilder {
//     pub fn new(device_name: &str) -> Self {
//         Self {
//             client_id: device_name.to_string(),
//         }
//     }
//
//     pub fn build_request(&self, request_payload: PayloadType) -> RequestBuilder {
//         return RequestBuilder {
//             header: self.build_header(String::new(),
//                                       HeaderType::Request(
//                                           RequestHeader {
//                                               cancel: false
//                                           })),
//             payload_type: request_payload
//         };
//     }
//
//     pub fn build_event(&self, event_payload: PayloadType) -> EventBuilder {
//         return EventBuilder {
//             header: self.build_header(String::new(), HeaderType::Event(EventHeader {})),
//             payload_type: event_payload
//         };
//     }
//
//     pub fn build_response(&self) -> ResponseBuilder {
//         return ResponseBuilder {
//             header: self.build_header(
//                 String::new(),
//                 HeaderType::Response(
//                     ResponseHeader {
//                         code: 0,
//                         message: String::new(),
//                         request_id: String::new(),
//                     }
//                 )),
//             payload_type: None
//         }
//     }
//
//     pub fn build_header(&self, to_id: String, header_type: HeaderType) -> Header {
//         Header {
//             id: Uuid::new_v4().to_string(),
//             from_id: self.client_id.clone(),
//             to_id,
//             time: Some(MessageBuilder::build_timestamp()),
//             header_type: Some(header_type),
//         }
//     }
//
//     pub fn build_timestamp() -> Timestamp {
//         MessageBuilder::from_system_time(&SystemTime::now())
//     }
//
//     pub fn from_system_time(timestamp: &SystemTime) -> Timestamp {
//         let mut ts = Timestamp {
//             seconds: 0,
//             nanos: 0,
//         };
//         if let Ok(d) = timestamp.duration_since(std::time::UNIX_EPOCH) {
//             ts.seconds = d.as_secs() as i64;
//             ts.nanos = d.subsec_nanos() as i32;
//         }
//         ts
//     }
//
//     pub fn to_system_time(timestamp: &Timestamp) -> SystemTime {
//         SystemTime::UNIX_EPOCH.add(
//             Duration::from_secs(timestamp.seconds as u64)
//             .add(Duration::from_nanos(timestamp.nanos as u64)))
//     }
// }

// #[cfg(test)]
// mod test {
//     use serde_json;
//
//     use crate::{DroneMode, HeaderType, LauncherStateEvent, LoadRequest, LoadResponse, MessageBuilder, PayloadType, ProtoBuilder, ResponseCode, SysId, Tube, TubeState};
//
//     #[test]
//     fn test_build_event() {
//         let message_builder = MessageBuilder::new("dorkus");
//         let message = message_builder.build_event(
//             PayloadType::LauncherStateEvent(LauncherStateEvent { launcher: None })).build();
//         assert_ne!(None, message.header);
//         assert_ne!(None, message.payload);
//
//         let payload = message.payload.unwrap();
//         assert_ne!(None, payload.payload_type);
//         assert_eq!(PayloadType::LauncherStateEvent(LauncherStateEvent { launcher: None }), payload.payload_type.unwrap());
//
//         let header = message.header.unwrap();
//         assert_ne!("", header.id);
//         assert_ne!(None, header.time);
//         assert_eq!("dorkus", header.from_id);
//     }
//
//     #[test]
//     fn test_build_request() {
//         let message_builder = MessageBuilder::new("dorkus");
//         let request = PayloadType::LoadRequest(LoadRequest { tube_id: 1, sys_id: Some(SysId::from("gcs_1")) });
//         let message = message_builder.build_request(request.clone()).build();
//         assert_ne!(None, message.header);
//         assert_ne!(None, message.payload);
//
//         let payload = message.payload.unwrap();
//         assert_ne!(None, payload.payload_type);
//         assert_eq!(request, payload.payload_type.unwrap());
//
//         let header = message.header.unwrap();
//         assert_ne!("", header.id);
//         assert_ne!(None, header.time);
//         assert_eq!("dorkus", header.from_id);
//     }
//
//     #[test]
//     fn test_build_response() {
//         let message_builder = MessageBuilder::new("dorkus");
//         let request = PayloadType::LoadRequest(LoadRequest { tube_id: 1, sys_id: Some(SysId::from("gcs_1")) });
//         let req_message = message_builder.build_request(request.clone()).build();
//
//         let response = PayloadType::LoadResponse(LoadResponse { });
//         let message = message_builder.build_response()
//             .for_request(req_message.header.as_ref().unwrap())
//             .with_result(Ok(()))
//             .with_payload(response.clone())
//             .build();
//         assert_ne!(None, message.header);
//         assert_ne!(None, message.payload);
//
//         let payload = message.payload.unwrap();
//         assert_ne!(None, payload.payload_type);
//         assert_eq!(response, payload.payload_type.unwrap());
//
//         let header = message.header.unwrap();
//         assert_ne!("", header.id);
//         assert_ne!(None, header.time);
//         assert_eq!("dorkus", header.from_id);
//
//         let header_type = header.header_type.unwrap();
//         match header_type {
//             HeaderType::Response(response) => {
//                 assert_eq!(req_message.header.unwrap().id, response.request_id);
//                 assert_eq!(ResponseCode::Ok as i32, response.code);
//                 assert_eq!("", response.message);
//             }
//             _ => {
//                 assert!(false, "Expected response header")
//             }
//         }
//
//         // error
//         let req_message = message_builder.build_request(request.clone()).build();
//         let message = message_builder.build_response()
//             .for_request(req_message.header.as_ref().unwrap())
//             .with_result(Err("busted".to_string()))
//             .with_payload(response.clone())
//             .build();
//
//         let header = message.header.unwrap();
//         let header_type = header.header_type.unwrap();
//         match header_type {
//             HeaderType::Response(response) => {
//                 assert_eq!(req_message.header.unwrap().id, response.request_id);
//                 assert_eq!(ResponseCode::Error as i32, response.code);
//                 assert_eq!("busted", response.message);
//             }
//             _ => {
//                 assert!(false, "Expected response header")
//             }
//         }
//
//         // empty
//         let req_message = message_builder.build_request(request.clone()).build();
//         let message = message_builder.build_response()
//             .for_request(req_message.header.as_ref().unwrap())
//             .with_result(Ok(()))
//             .build();
//
//         assert_eq!(None, message.payload);
//         let header = message.header.unwrap();
//         let header_type = header.header_type.unwrap();
//         match header_type {
//             HeaderType::Response(response) => {
//                 assert_eq!(req_message.header.unwrap().id, response.request_id);
//                 assert_eq!(ResponseCode::Ok as i32, response.code);
//                 assert_eq!("", response.message);
//             }
//             _ => {
//                 assert!(false, "Expected response header")
//             }
//         }
//     }
//
//     #[test]
//     fn test_tub_load_unload() {
//         let mut tube = Tube {
//             id: 0,
//             state: 0,
//             sys_id: None,
//             charge: 0
//         };
//
//         let sys_id = SysId::from("gcs1_1");
//         tube.load(&sys_id);
//
//         assert_eq!(&sys_id, tube.sys_id.as_ref().unwrap());
//         assert_eq!(TubeState::Loaded as i32, tube.state);
//
//         tube.unload();
//         assert_eq!(None, tube.sys_id);
//         assert_eq!(TubeState::NotLoaded as i32, tube.state);
//     }
//
//     #[test]
//     fn test_sys_id_string_conversions() {
//         let sys_id0 = SysId {
//             proxy_id: "gcs1".to_string(),
//             sys_id: 1
//         };
//
//         let sys_id = SysId::from("gcs1_1");
//         let sys_id1 = SysId::from("gcs1_2");
//         let sys_id2 = SysId::from(sys_id.to_string());
//         assert_eq!(1, sys_id.sys_id);
//         assert_eq!("gcs1", sys_id.proxy_id);
//         assert_eq!("gcs1_1", sys_id.to_string());
//         assert_eq!(sys_id0, sys_id);
//         assert_eq!(sys_id2, sys_id);
//         assert_ne!(sys_id1, sys_id2);
//         assert_eq!(2, sys_id1.sys_id);
//         assert_eq!("gcs1", sys_id1.proxy_id);
//
//         let sys_id3 = SysId::from("This isn't good");
//         assert_eq!("This isn't good_0", sys_id3.to_string());
//     }
//
//     #[test]
//     fn test_drone_mode_to_from_primitive() {
//         assert_eq!(DroneMode::from_i32(1).unwrap(), DroneMode::Arming);
//     }
//
//     #[test]
//     fn test_serde_json_partial() {
//         let partial = "{\"SetLauncherRequest\": {\"launcher\": {\"safeDistance\": 5, \"location\": {\"point\": {\"latitude\": 40.008663, \"longitude\": -74.59827, \"altitude\": 1}}}}}";
//         let payload = serde_json::from_str::<PayloadType>(partial).unwrap();
//         println!("{:?}", payload);
//     }
// }
