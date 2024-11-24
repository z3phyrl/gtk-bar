use async_std::task::sleep;
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
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::env::var;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc as std_mpsc;
use std::time::Duration;
use tokio::sync::broadcast as tokio_broadcast;
use tokio::sync::mpsc as tokio_mpsc;

use crate::hyprland::Hyprland;

#[derive(Deserialize, PartialEq)]
struct WorkspaceInfo {
    id: u32,
    name: String,
}

#[derive(Deserialize)]
struct Client {
    workspace: WorkspaceInfo,
    hidden: bool,
    floating: bool,
}

#[derive(Clone)]
pub struct Root {
    root: Overlay,
    bg: Revealer,
    left: Box,
    center: Box,
    right: Box,
}

impl Root {
    pub fn widget(&self) -> Overlay {
        self.root.clone()
    }
    pub fn left<W>(&mut self, widgets: Vec<&W>)
    where
        W: IsA<Widget>,
    {
        for widget in widgets {
            self.left.append(widget);
        }
    }
    pub fn center<W>(&mut self, widgets: Vec<&W>)
    where
        W: IsA<Widget>,
    {
        for widget in widgets {
            self.center.append(widget);
        }
    }
    pub fn right<W>(&mut self, widgets: Vec<&W>)
    where
        W: IsA<Widget>,
    {
        for widget in widgets {
            self.right.append(widget);
        }
    }
    pub fn transparent(&mut self, transparent: bool) {
        self.bg.set_reveal_child(transparent);
    }
    pub fn transparency(&self) -> bool {
        self.bg.reveals_child()
    }
    pub fn spacing(&mut self, spacing: i32) {
        self.left.set_spacing(spacing);
        self.center.set_spacing(spacing);
        self.right.set_spacing(spacing);
    }
}

pub trait HyprlandRootExt {
    fn root(&self) -> Root;
}

impl HyprlandRootExt for Hyprland {
    fn root(&self) -> Root {
        let bg = Revealer::builder()
            .transition_type(RevealerTransitionType::Crossfade)
            .transition_duration(500)
            .child(&Box::builder().css_classes(["bg"]).build())
            .reveal_child(true)
            .build();
        let background = bg.clone();
        let root = Overlay::builder().child(&bg).build();
        let left = Box::new(Orientation::Horizontal, 0);
        let center = Box::new(Orientation::Horizontal, 0);
        let right = Box::new(Orientation::Horizontal, 0);
        let content = CenterBox::builder()
            .start_widget(&left)
            .center_widget(&center)
            .end_widget(&right)
            .build();
        root.add_overlay(&content);
        let mut listener = self.listener();
        let mut controller = self.controller();
        println!("what");
        spawn_future_local(async move {
            loop {
                if let Ok(event) = listener.recv().await {
                    let mut event = event.split(">>");
                    let name = event.next();
                    match name {
                        Some("workspacev2")
                        | Some("openwindow")
                        | Some("closewindow")
                        | Some("changefloatingmode") => {
                            let clients: Vec<Client> =
                                from_str(&controller.ctl("j/clients")).unwrap();
                            let current_workspace = if name == Some("workspacev2") {
                                if let Some(data) = event.next() {
                                    let data: Vec<&str> = data.split(",").collect();
                                    WorkspaceInfo {
                                        id: data[0].parse::<u32>().unwrap(),
                                        name: data[1].to_string(),
                                    }
                                } else {
                                    from_str(&controller.ctl("j/activeworkspace")).unwrap()
                                }
                            } else {
                                from_str(&controller.ctl("j/activeworkspace")).unwrap()
                            };
                            if clients.iter().filter(|c| c.workspace == current_workspace && !c.hidden && !c.floating).count() == 0 {
                                background.set_reveal_child(false);
                            } else {
                                background.set_reveal_child(true);
                            }
                        }
                        _ => {}
                    }
                }
            }
        });
        Root {
            root,
            bg,
            left,
            center,
            right,
        }
    }
}
