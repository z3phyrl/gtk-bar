use async_broadcast::{broadcast, InactiveReceiver, Receiver, Sender};
use lazy_static::lazy_static;
use std::{
    boxed::Box,
    env::var,
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
};

lazy_static! {
    static ref HYPRLAND_INSTANCE_SIGNATURE: String = var("HYPRLAND_INSTANCE_SIGNATURE").unwrap();
    static ref XDG_RUNTIME_DIR: PathBuf = var("XDG_RUNTIME_DIR").unwrap().into();
    static ref HYPRDIR: PathBuf = XDG_RUNTIME_DIR
        .join("hypr")
        .join(HYPRLAND_INSTANCE_SIGNATURE.as_str());
    static ref CONTROL_SOCK_PATH: PathBuf = HYPRDIR.join(".socket.sock");
    static ref EVENT_SOCK_PATH: PathBuf = HYPRDIR.join(".socket2.sock");
}

pub struct Hyprland {
    events: UnixStream,
    sender: Sender<String>,
    receiver: InactiveReceiver<String>,
}

pub fn new() -> Hyprland {
    Hyprland::new()
}

pub fn ctl(req: &str) -> String {
    let mut stream = UnixStream::connect(CONTROL_SOCK_PATH.as_path()).unwrap();
    write!(stream, "{req}").unwrap();
    let mut buf = Vec::new();
    let mut reader = BufReader::new(stream);
    reader.read_until(b';', &mut buf).unwrap();
    String::from_utf8(buf).unwrap()
}

impl Hyprland {
    pub fn new() -> Self {
        let events = UnixStream::connect(EVENT_SOCK_PATH.as_path()).unwrap();
        let (sender, receiver) = broadcast(1024);
        Self {
            events,
            sender,
            receiver: receiver.deactivate(),
        }
    }
    pub async fn listen(&mut self) {
        let events = self.events.try_clone().unwrap();
        let reader = BufReader::new(events);
        for line in reader.lines() {
            if let Ok(line) = line {
                // println!("sender {:?}", self.sender.len());
                self.sender.broadcast_direct(line).await.unwrap();
            }
        }
    }
    pub fn listener(&self) -> Receiver<String> {
        self.sender.new_receiver()
    }
}
