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

#[derive(Deserialize, Debug, Hash, Eq, PartialEq, Clone)]
struct WorkspaceInfo {
    id: u32,
    name: String,
}

#[derive(Debug, Clone)]
struct Workspace {
    info: WorkspaceInfo,
    widget: Box,
    slidein: Revealer,
    crossfade: Revealer,
    expander: Revealer,
}

impl Workspace {
    fn new(info: WorkspaceInfo) -> Self {
        let widget = Box::new(Orientation::Horizontal, 0);
        let slidein = Revealer::builder()
            .transition_type(RevealerTransitionType::SlideLeft)
            .transition_duration(250)
            .build();
        let crossfade = Revealer::builder()
            .transition_type(RevealerTransitionType::Crossfade)
            .transition_duration(1000)
            .build();
        let main = Box::new(Orientation::Horizontal, 0);
        let anchor = Box::default();
        let expander = Revealer::builder()
            .transition_type(RevealerTransitionType::SlideLeft)
            .transition_duration(1000)
            .build();
        let expand = Box::default();
        main.add_css_class("workspace");
        anchor.add_css_class("anchor");
        expand.add_css_class("w");
        widget.append(&slidein);
        slidein.set_child(Some(&crossfade));
        crossfade.set_child(Some(&main));
        main.append(&anchor);
        main.append(&expander);
        expander.set_child(Some(&expand));
        Self {
            info,
            widget,
            slidein,
            crossfade,
            expander,
        }
    }
    async fn reveal(&self, reveal: bool) {
        if reveal {
            self.slidein.set_transition_duration(250);
            self.crossfade.set_transition_duration(500);
            self.slidein.set_reveal_child(reveal);
            sleep(Duration::from_millis(100)).await;
            self.crossfade.set_reveal_child(reveal);
        } else {
            self.slidein.set_transition_duration(250);
            self.crossfade.set_transition_duration(500);
            self.crossfade.set_reveal_child(reveal);
            sleep(Duration::from_millis(250)).await;
            self.slidein.set_reveal_child(reveal);
        }
    }
    fn revealed(&self) -> bool {
        self.slidein.reveals_child() || self.crossfade.reveals_child()
    }
    fn expand(&self, expand: bool) {
        self.expander.set_reveal_child(expand);
    }
}

pub trait HyprlandWorkspacesExt {
    fn workspaces(&self) -> Box;
}

impl HyprlandWorkspacesExt for Hyprland {
    fn workspaces(&self) -> Box {
        let widget = Box::new(Orientation::Horizontal, 0);
        let workspaces_widget = widget.clone();
        let mut events = self.listener();
        let mut controller = self.controller();
        let mut workspaces: HashMap<WorkspaceInfo, Workspace> = HashMap::new();
        spawn_future_local(async move {
            for info in from_str::<Vec<WorkspaceInfo>>(&controller.ctl("j/workspaces")).unwrap() {
                let workspace = Workspace::new(info.clone());
                let before: Vec<(&WorkspaceInfo, &Workspace)> = workspaces
                    .iter()
                    .filter(|w| w.0.id == info.id - 1)
                    .collect();
                if info.id == 1 {
                    workspaces_widget.insert_child_after(&workspace.widget, None::<&Box>);
                } else if let Some(before) = before.last() {
                    workspaces_widget.insert_child_after(&workspace.widget, Some(&before.1.widget));
                } else {
                    workspaces_widget.append(&workspace.widget);
                }
                workspace.reveal(true).await;
                workspaces.insert(info, workspace);
            }
            if let Some(current) =
                workspaces.get(&from_str(&controller.ctl("j/activeworkspace")).unwrap())
            {
                current.expand(true);
            }
            loop {
                if let Ok(event) = events.recv().await {
                    let mut event = event.split(">>");
                    match event.next() {
                        Some("workspacev2") => {
                            println!("workspacev2");
                            if let Some(data) = event.next() {
                                let data: Vec<&str> = data.split(",").collect();
                                let current = WorkspaceInfo {
                                    id: data[0].parse::<u32>().unwrap(),
                                    name: data[1].to_string(),
                                };
                                for (info, workspace) in &workspaces {
                                    workspace.expand(*info == current);
                                }
                            }
                        }
                        Some("createworkspacev2") => {
                            println!("createworkspacev2");
                            if let Some(data) = event.next() {
                                let data: Vec<&str> = data.split(",").collect();
                                let info = WorkspaceInfo {
                                    id: data[0].parse::<u32>().unwrap(),
                                    name: data[1].to_string(),
                                };
                                let workspace = Workspace::new(info.clone());
                                let widget = workspace.clone();
                                let before: Option<(&WorkspaceInfo, &Workspace)> = workspaces
                                    .iter()
                                    .filter(|w| w.0.id < info.id)
                                    .max_by_key(|(i, _)| i.id);
                                if info.id == 1 {
                                    workspaces_widget
                                        .insert_child_after(&workspace.widget, None::<&Box>);
                                } else if let Some(before) = before {
                                    workspaces_widget.insert_child_after(
                                        &workspace.widget,
                                        Some(&before.1.widget),
                                    );
                                } else {
                                    workspaces_widget.append(&workspace.widget);
                                }
                                widget.reveal(true).await;
                                workspaces.insert(info, workspace.clone());
                            }
                        }
                        Some("destroyworkspacev2") => {
                            println!("destroyworkspacev2");
                            if let Some(data) = event.next() {
                                let data: Vec<&str> = data.split(",").collect();
                                let info = WorkspaceInfo {
                                    id: data[0].parse::<u32>().unwrap(),
                                    name: data[1].to_string(),
                                };
                                let ws = workspaces.clone();
                                let workspaces_widget = workspaces_widget.clone();
                                workspaces.remove(&info);
                                spawn_future_local(async move {
                                    if let Some(widget) = ws.get(&info) {
                                        if widget.revealed() {
                                            widget.reveal(false).await;
                                            sleep(Duration::from_millis(150)).await;
                                        }
                                        workspaces_widget.remove(&widget.widget);
                                    }
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
        });
        widget
    }
}
