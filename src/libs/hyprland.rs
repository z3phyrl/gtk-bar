use async_broadcast::{broadcast, InactiveReceiver, Receiver, Sender};
use std::{
    env::var,
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
};

pub struct Hyprland {
    hypr_dir: PathBuf,
    events: UnixStream,
    sender: Sender<String>,
    receiver: InactiveReceiver<String>,
}

pub fn new() -> Hyprland {
    let hyprland_instance_signature = var("HYPRLAND_INSTANCE_SIGNATURE").unwrap();
    let xdg_runtime_dir = var("XDG_RUNTIME_DIR").unwrap();
    let mut hypr_dir = PathBuf::from(xdg_runtime_dir);
    hypr_dir.push("hypr");
    hypr_dir.push(hyprland_instance_signature);
    let events_path = hypr_dir.join(".socket2.sock");
    let events = UnixStream::connect(events_path).unwrap();
    let (sender, receiver) = broadcast(1024);
    Hyprland {
        hypr_dir,
        events,
        sender,
        receiver: receiver.deactivate(),
    }
}

#[derive(Clone)]
pub struct Controller {
    socket_path: PathBuf,
}

impl Controller {
    pub fn from_socket_path(socket_path: PathBuf) -> Self {
    Controller {
            socket_path,
        }
    }
    pub fn ctl(&mut self, req: &str) -> String {
        let mut stream = UnixStream::connect(self.socket_path.clone()).unwrap();
        write!(stream, "{req}").unwrap();
        let mut buf = Vec::new();
        let mut reader = BufReader::new(stream);
        reader.read_until(b';', &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }
}

impl Hyprland {
    pub fn controller(&self) -> Controller {
        Controller {
            socket_path: self.hypr_dir.join(".socket.sock"),
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
