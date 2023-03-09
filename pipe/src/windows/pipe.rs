use std::error::Error;
use std::io;
use tokio::net::windows::named_pipe;

pub const INPUT_PIPE_NAME: &str = r"\\.\rkvm2.pipe";

pub type ClientPipeStream = NamedPipeClient;
pub type ServerPipeStream = NamedPipeServer;

pub async fn connect(name: &str) -> io::Result<ClientPipeStream> {
}

pub async fn accept(name: &str, gid: u32) -> ServerPipeStream {
    let server = named_pipe::ServerOptions::new().create(name)?;
}
