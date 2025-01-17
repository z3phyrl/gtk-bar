use crate::*;
use async_broadcast::{broadcast, InactiveReceiver, Receiver};
use hyprland::ctl;

#[derive(Deserialize, PartialEq)]
struct WorkspaceInfo {
    id: i32,
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
    pub left: Box,
    pub center: Box,
    pub right: Box,
    recv: InactiveReceiver<bool>,
}

impl Root {
    pub fn widget(&self) -> Overlay {
        self.root.clone()
    }
    pub fn left<W>(&mut self, widget: &W)
    where
        W: IsA<Widget>,
    {
        self.left.append(widget);
    }
    pub fn center<W>(&mut self, widget: &W)
    where
        W: IsA<Widget>,
    {
        self.center.append(widget);
    }
    pub fn right<W>(&mut self, widget: &W)
    where
        W: IsA<Widget>,
    {
        self.right.append(widget);
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
    pub fn listen(&self) -> Receiver<bool> {
        self.recv.activate_cloned()
    }
}

pub fn new(mut event_listener: Receiver<String>) -> Root {
    let bg = Revealer::builder()
        .transition_type(Crossfade)
        .transition_duration(500)
        .child(&Box::builder().css_classes(["bg"]).build())
        .build();
    let background = bg.clone();
    let root = Overlay::builder().child(&bg).build();
    let left = Box::new(Horizontal, 0);
    let center = Box::new(Horizontal, 0);
    let right = Box::new(Horizontal, 0);
    let content = CenterBox::builder()
        .start_widget(&left)
        .center_widget(&center)
        .end_widget(&right)
        .build();
    root.add_overlay(&content);
    let (snd, recv) = broadcast(64);
    spawn_future_local(async move {
        loop {
            if let Ok(event) = event_listener.recv().await {
                let mut event = event.split(">>");
                let name = event.next();
                match name {
                    Some("workspacev2")
                    | Some("openwindow")
                    | Some("closewindow")
                    | Some("changefloatingmode") => {
                        let clients: Vec<Client> = from_str(&ctl("j/clients")).unwrap();
                        let current_workspace = if name == Some("workspacev2") {
                            if let Some(data) = event.next() {
                                let data: Vec<&str> = data.split(",").collect();
                                WorkspaceInfo {
                                    id: data[0].parse::<i32>().unwrap(),
                                    name: data[1].to_string(),
                                }
                            } else {
                                from_str(&ctl("j/activeworkspace")).unwrap()
                            }
                        } else {
                            from_str(&ctl("j/activeworkspace")).unwrap()
                        };
                        if clients
                            .iter()
                            .filter(|c| {
                                c.workspace == current_workspace && !c.hidden && !c.floating
                            })
                            .count()
                            == 0
                        {
                            background.set_reveal_child(false);
                            snd.broadcast(false).await.unwrap();
                        } else {
                            background.set_reveal_child(true);
                            snd.broadcast(true).await.unwrap();
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
        recv: recv.deactivate(),
    }
}
