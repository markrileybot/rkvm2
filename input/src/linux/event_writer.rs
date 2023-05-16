use std::io::{Error, ErrorKind};
use std::mem::MaybeUninit;
use std::ops::RangeInclusive;

use rkvm2_proto::InputEvent;

use crate::linux::device_id;
use crate::linux::glue::{self, __s32, __u16, input_event, libevdev, libevdev_uinput, timeval};

pub struct EventWriter {
    evdev: *mut libevdev,
    uinput: *mut libevdev_uinput,
}

impl EventWriter {
    pub async fn new() -> Result<Self, Error> {
        tokio::task::spawn_blocking(|| -> Result<Self, Error> {
            return Self::new_sync();
        })
        .await?
    }

    fn new_sync() -> Result<Self, Error> {
        let evdev = unsafe { glue::libevdev_new() };
        if evdev.is_null() {
            return Err(Error::new(ErrorKind::Other, "Failed to create device"));
        }

        if let Err(err) = unsafe { setup_evdev(evdev) } {
            unsafe {
                glue::libevdev_free(evdev);
            }

            return Err(err);
        }

        let mut uinput = MaybeUninit::uninit();
        let ret = unsafe {
            glue::libevdev_uinput_create_from_device(
                evdev,
                glue::libevdev_uinput_open_mode_LIBEVDEV_UINPUT_OPEN_MANAGED,
                uinput.as_mut_ptr(),
            )
        };

        if ret < 0 {
            unsafe { glue::libevdev_free(evdev) };
            return Err(Error::new(
                Error::from_raw_os_error(-ret).kind(),
                format!("Failed to create from device ({})", ret),
            ));
        }

        let uinput = unsafe { uinput.assume_init() };
        let mut event_writer = Self { evdev, uinput };

        // send to 0, 0?
        event_writer.write_raw(input_event {
            time: timeval { tv_sec: 0, tv_usec: 0 },
            type_: glue::EV_ABS as _,
            code: glue::ABS_X as _,
            value: 0,
        })?;

        event_writer.write_raw(input_event {
            time: timeval { tv_sec: 0, tv_usec: 0 },
            type_: glue::EV_ABS as _,
            code: glue::ABS_Y as _,
            value: 0,
        })?;

        Ok(event_writer)
    }

    pub async fn write(&mut self, event: InputEvent) -> Result<(), Error> {
        self.write_raw(event.into())
    }

    pub(crate) fn write_raw(&mut self, event: input_event) -> Result<(), Error> {
        self.write_raw_0(event.type_, event.code, event.value)?;
        self.write_raw_0(glue::EV_SYN as _, glue::SYN_REPORT as _, 0)?;
        Ok(())
    }

    fn write_raw_0(&mut self, r#type: __u16, code: __u16, value: __s32) -> Result<(), Error> {
        // As far as tokio is concerned, the FD never becomes ready for writing, so just write it normally.
        // If an error happens, it will be propagated to caller and the FD is opened in nonblocking mode anyway,
        // so it shouldn't be an issue.

        let ret = unsafe {
            glue::libevdev_uinput_write_event(
                self.uinput as *const _,
                r#type as _,
                code as _,
                value,
            )
        };

        if ret < 0 {
            return Err(Error::from_raw_os_error(-ret));
        }
        Ok(())
    }
}

impl Drop for EventWriter {
    fn drop(&mut self) {
        unsafe {
            glue::libevdev_uinput_destroy(self.uinput);
            glue::libevdev_free(self.evdev);
        }
    }
}

unsafe impl Send for EventWriter {}

const TYPES: &[(u32, &[RangeInclusive<u32>])] = &[
    (glue::EV_SYN, &[glue::SYN_REPORT..=glue::SYN_REPORT]),
    (glue::EV_REL, &[0..=glue::REL_MAX]),
    (glue::EV_KEY, &[0..=/*glue::KEY_MAX*/565]),
];

unsafe fn setup_evdev(evdev: *mut libevdev) -> Result<(), Error> {
    glue::libevdev_set_name(evdev, b"rkvm2\0".as_ptr() as *const _);
    glue::libevdev_set_id_vendor(evdev, device_id::VENDOR as _);
    glue::libevdev_set_id_product(evdev, device_id::PRODUCT as _);
    glue::libevdev_set_id_version(evdev, device_id::VERSION as _);
    glue::libevdev_set_id_bustype(evdev, glue::BUS_USB as _);

    for (r#type, codes) in TYPES.iter().copied() {
        let ret = glue::libevdev_enable_event_type(evdev, r#type);
        if ret < 0 {
            return Err(Error::new(
                Error::from_raw_os_error(-ret).kind(),
                format!("Failed to enable event type {} ({})", r#type, ret),
            ));
        }

        for code in codes.iter().cloned().flatten() {
            let ret = glue::libevdev_enable_event_code(evdev, r#type, code, std::ptr::null_mut());
            if ret < 0 {
                return Err(Error::new(
                    Error::from_raw_os_error(-ret).kind(),
                    format!(
                        "Failed to enable event type {} code {} ({})",
                        r#type, code, ret
                    ),
                ));
            }
        }
    }

    Ok(())
}
