use crate::*;
use async_broadcast::Receiver;
use hyprland::ctl;

#[derive(Deserialize, Debug, Hash, Eq, PartialEq, Clone)]
struct WorkspaceInfo {
    id: i32,
    name: String,
}

#[derive(Debug, Clone)]
struct Workspace {
    special: bool,
    widget: Box,
    slidein: Revealer,
    crossfade: Revealer,
    expander: Revealer,
}

impl Workspace {
    fn new(info: &WorkspaceInfo, special: bool) -> Self {
        let widget = Box::new(Horizontal, 0);
        let slidein = Revealer::builder()
            .transition_type(SlideLeft)
            .transition_duration(250)
            .build();
        let crossfade = Revealer::builder()
            .transition_type(Crossfade)
            .transition_duration(500)
            .build();
        let main = Box::new(Horizontal, 0);
        let anchor = Box::default();
        let expander = Revealer::builder()
            .transition_type(SlideLeft)
            .transition_duration(1000)
            .build();
        let expand = Box::default();
        if special {
            main.add_css_class("specialworkspace")
        } else {
            main.add_css_class("workspace");
        }
        anchor.add_css_class("anchor");
        expand.add_css_class("expand");
        widget.append(&slidein);
        slidein.set_child(Some(&crossfade));
        crossfade.set_child(Some(&main));
        main.append(&anchor);
        main.append(&expander);
        expander.set_child(Some(&expand));
        let lclick = GestureClick::new();
        let id = info.id;
        lclick.connect_pressed(move |click, count, x, y| {
            ctl(&format!("dispatch workspace {}", id));
        });
        widget.add_controller(lclick);
        Self {
            special,
            widget,
            slidein,
            crossfade,
            expander,
        }
    }
    async fn reveal(&self, reveal: bool) {
        if reveal {
            self.slidein.set_reveal_child(reveal);
            sleep(Duration::from_millis(100)).await;
            self.crossfade.set_reveal_child(reveal);
        } else {
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

pub fn new(mut event_listener: Receiver<String>) -> Box {
    let widget = Box::new(Horizontal, 0);
    let workspaces_widget = widget.clone();
    let mut workspaces: HashMap<WorkspaceInfo, Workspace> = HashMap::new();
    spawn_future_local(async move {
        for info in from_str::<Vec<WorkspaceInfo>>(&hyprland::ctl("j/workspaces")).unwrap() {
            let workspace = Workspace::new(&info, false);
            let before: Option<(&WorkspaceInfo, &Workspace)> = workspaces
                .iter()
                .filter(|w| w.0.id < info.id)
                .max_by_key(|(i, _)| i.id);
            if info.id < 0 {
                let workspace = Workspace::new(&info, true);
                workspaces_widget.insert_child_after(&workspace.widget, None::<&Box>);
                workspace.reveal(true).await;
                workspaces.insert(info, workspace.clone());
                continue;
            } else if let Some(before) = before {
                workspaces_widget.insert_child_after(&workspace.widget, Some(&before.1.widget));
            } else if info.id == 1 {
                workspaces_widget.insert_child_after(&workspace.widget, None::<&Box>);
            } else {
                workspaces_widget.append(&workspace.widget);
            }

            workspace.reveal(true).await;
            workspaces.insert(info, workspace.clone());
        }
        if let Some(current) = workspaces.get(&from_str(&ctl("j/activeworkspace")).unwrap()) {
            current.expand(true);
        }
        loop {
            if let Ok(event) = event_listener.recv().await {
                let mut event = event.split(">>");
                match event.next() {
                    Some("workspacev2") => {
                        if let Some(data) = event.next() {
                            let data: Vec<&str> = data.split(",").collect();
                            // println!("{data:?}");
                            let current = WorkspaceInfo {
                                id: data[0].parse::<i32>().unwrap(),
                                name: data[1].to_string(),
                            };
                            for (info, workspace) in &workspaces {
                                if !workspace.special {
                                    workspace.expand(*info == current);
                                }
                            }
                        }
                    }
                    Some("createworkspacev2") => {
                        // println!("::createworkspacev2");
                        if let Some(data) = event.next() {
                            let data: Vec<&str> = data.split(",").collect();
                            let info = WorkspaceInfo {
                                id: data[0].parse::<i32>().unwrap(),
                                name: data[1].to_string(),
                            };
                            let workspace = Workspace::new(&info, false);
                            let before: Option<(&WorkspaceInfo, &Workspace)> = workspaces
                                .iter()
                                .filter(|w| w.0.id < info.id)
                                .max_by_key(|(i, _)| i.id);
                            if info.id < 0 {
                                let workspace = Workspace::new(&info, true);
                                workspaces_widget
                                    .insert_child_after(&workspace.widget, None::<&Box>);
                                workspace.reveal(true).await;
                                workspaces.insert(info, workspace.clone());
                                continue;
                            } else if let Some(before) = before {
                                workspaces_widget
                                    .insert_child_after(&workspace.widget, Some(&before.1.widget));
                            } else if info.id == 1 {
                                workspaces_widget
                                    .insert_child_after(&workspace.widget, None::<&Box>);
                            } else {
                                workspaces_widget.append(&workspace.widget);
                            }
                            workspace.reveal(true).await;
                            workspaces.insert(info, workspace.clone());
                        }
                    }
                    Some("destroyworkspacev2") => {
                        // println!("::destroyworkspacev2");
                        if let Some(data) = event.next() {
                            let data: Vec<&str> = data.split(",").collect();
                            let info = WorkspaceInfo {
                                id: data[0].parse::<i32>().unwrap(),
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
                    Some("activespecial") => {
                        if let Some(data) = event.next() {
                            let data: Vec<&str> = data.split(",").collect();
                            let workspace_name = data[0];
                            let mut open = true;
                            if workspace_name.is_empty() {
                                open = false
                            }
                            for (_, workspace) in &workspaces {
                                if workspace.special {
                                    workspace.expand(open);
                                }
                            }
                        }
                    }
                    _e => {
                        // println!("{e:?}")
                    }
                }
            }
        }
    });
    widget
}
