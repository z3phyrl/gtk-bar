use chrono::Local;
use gtk::gdk;
use gtk::prelude::*;
use gtk::{
    gio,
    glib::{self, idle_add_local, spawn_future_local, timeout_add_local, ControlFlow, MainContext},
    Application, ApplicationWindow, Box, Button, CenterBox, CssProvider, EventControllerMotion,
    GestureClick, Label, Orientation, Overlay, Revealer, RevealerTransitionType, Widget,
};
use gtk4 as gtk;
use serde::Deserialize;
use serde_json::from_str;
use std::cell::RefCell;
use std::env::var;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

#[derive(Debug)]
struct CurrentWorkspace {
    id: i32,
    tiled: bool,
}

#[derive(Debug)]
enum WorkspacesData {
    Current(CurrentWorkspace),
    Workspaces(Vec<WorkspaceInfo>),
}

#[derive(Deserialize, Debug)]
struct WorkspaceInfo {
    id: u32,
    name: String,
    monitor: String,
    monitorID: u32,
    windows: u32,
    hasfullscreen: bool,
    lastwindow: String,
    lastwindowtitle: String,
}

#[derive(Deserialize, Debug)]
struct Workspace {
    id: i32,
    name: String,
}

#[derive(Deserialize, Debug)]
struct Client {
    hidden: bool,
    workspace: Workspace,
    floating: bool,
} // incomplete idk what's the proper type

pub struct Hyprland {
    hypr_dir: PathBuf,
    events: UnixStream,
    listeners: Vec<Sender<String>>,
}

pub fn new() -> Hyprland {
    let hyprland_instance_signature = var("HYPRLAND_INSTANCE_SIGNATURE").unwrap();
    let xdg_runtime_dir = var("XDG_RUNTIME_DIR").unwrap();
    let mut hypr_dir = PathBuf::from(xdg_runtime_dir);
    hypr_dir.push("hypr");
    hypr_dir.push(hyprland_instance_signature);
    let events_path = hypr_dir.join(".socket2.sock");
    let events = UnixStream::connect(events_path).unwrap();
    let listeners = Vec::new();
    Hyprland {
        hypr_dir,
        events,
        listeners,
    }
}

pub struct Controller {
    socket_path: PathBuf,
}

impl Controller {
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
    pub fn listen(&mut self) {
        let events = self.events.try_clone().unwrap();
        let reader = BufReader::new(events);
        for line in reader.lines() {
            if let Ok(line) = line {
                for listener in self.listeners.clone() {
                    println!("!{line:?}");
                    let a = listener.send(line.clone());
                    match a {
                        Ok(_) => {}
                        Err(a) => {
                            eprintln!("{a}")
                        }
                    }
                }
            }
        }
    }
    pub fn listener(&mut self) -> Receiver<String> {
        let (sendr, recvr) = channel();
        self.listeners.push(sendr);
        recvr
    }
    pub fn workspaces(&mut self) -> Box {
        let widget = Box::new(Orientation::Horizontal, 0);
        let workspace = Label::new(Some("Hello"));
        widget.append(&workspace);
        let mut listener = self.listener();
        let mut controller = self.controller();
        let (sendr, mut recvr) = tokio::sync::mpsc::channel::<WorkspacesData>(1);
        let main_ctx = MainContext::new();
        tokio::task::spawn(async move {
            loop {
                if let Ok(event) = listener.recv() {
                    sendr
                        .send(WorkspacesData::Current(CurrentWorkspace {
                            id: 1,
                            tiled: true,
                        }))
                        .await
                        .unwrap();
                }
            }
        });
        spawn_future_local(async move {
            loop {
                if let Some(data) = recvr.recv().await {
                    match data {
                        WorkspacesData::Current(current) => {
                            println!("sesbian lex");
                            workspace.set_label(&format!("{current:?}"));
                        }
                        WorkspacesData::Workspaces(workspaces) => {}
                    }
                };
            }
        });
        widget
    }
}
