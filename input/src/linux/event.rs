use crate::linux::glue::{self, input_event, timeval};
use rkvm2_proto::input_event::InputEventType;
use rkvm2_proto::{InputEvent, KeyEvent, MouseMoveEvent};

pub(crate) struct InputEventAdapter;
impl InputEventAdapter {
    pub(crate) fn to_raw(e: InputEvent) -> input_event {
        let (type_, code, value) = match e.input_event_type.unwrap() {
            InputEventType::Key(e) => (glue::EV_KEY as _, e.key as u16, if e.down { 1 } else { 0 }),
            InputEventType::Button(e) => (
                glue::EV_KEY as _,
                e.button as u16,
                if e.down { 1 } else { 0 },
            ),
            InputEventType::Wheel(e) => (glue::EV_REL as _, glue::REL_WHEEL as _, e.delta),
            InputEventType::X(e) => (glue::EV_REL as _, glue::REL_X as _, e.delta),
            InputEventType::Y(e) => (glue::EV_REL as _, glue::REL_Y as _, e.delta),
        };

        input_event {
            type_,
            code,
            value,
            time: timeval {
                tv_sec: 0,
                tv_usec: 0,
            },
        }
    }

    pub(crate) fn from_raw(raw: input_event) -> Option<InputEvent> {
        let input_event_type = match (raw.type_ as _, raw.code as _, raw.value) {
            (glue::EV_REL, glue::REL_WHEEL, value) => Some(InputEventType::Wheel(MouseMoveEvent {
                delta: value as i32,
            })),
            (glue::EV_REL, glue::REL_X, value) => Some(InputEventType::X(MouseMoveEvent {
                delta: value as i32,
            })),
            (glue::EV_REL, glue::REL_Y, value) => Some(InputEventType::Y(MouseMoveEvent {
                delta: value as i32,
            })),
            (glue::EV_KEY, code, 0) => Some(InputEventType::Key(KeyEvent {
                down: false,
                key: code as i32,
            })),
            (glue::EV_KEY, code, 1) => Some(InputEventType::Key(KeyEvent {
                down: true,
                key: code as i32,
            })),
            _ => None,
        };

        if input_event_type.is_none() {
            return None;
        }
        return Some(InputEvent { input_event_type });
    }
}
