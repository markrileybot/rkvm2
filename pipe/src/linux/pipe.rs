extern crate core;

use std::{fs, io};
use std::fs::Permissions;
use std::io::ErrorKind::AddrInUse;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener as StdUnixListener;

use nix::unistd::{chown, Gid};
use tokio::net::{UnixListener, UnixStream};

pub const INPUT_PIPE_NAME: &str = r"/var/run/rkvm2.sock";

pub type ClientPipeStream = UnixStream;
pub type ServerPipeStream = UnixStream;

pub async fn connect(name: &str) -> io::Result<ClientPipeStream> {
    UnixStream::connect(name).await
}

pub async fn accept(name: &str, gid: u32) -> ServerPipeStream {
    loop {
        match StdUnixListener::bind(name) {
            Ok(listener) => {
                listener
                    .set_nonblocking(true)
                    .expect("Failed to set non blocking");
                fs::set_permissions(name, Permissions::from_mode(0o770))
                    .expect("Failed to change perms");
                chown(name, None, Some(Gid::from_raw(gid)))
                    .expect("Failed to change ownership");

                match UnixListener::from_std(listener) {
                    Ok(listener) => match listener.accept().await {
                        Ok((stream, _addr)) => {
                            return stream;
                        }
                        Err(e) => {
                            log::warn!("Accept failed {}", e);
                        }
                    },
                    Err(e) => {
                        panic!("Failed to bind to socket {}", e);
                    }
                }
            }
            Err(e) => {
                if e.kind() == AddrInUse {
                    fs::remove_file(name)
                        .expect(format!("Failed to remove existing socket {}", e).as_str());
                } else {
                    panic!("Failed to bind to socket {}", e);
                }
            }
        }
    }
}
