use crate::*;
use libs::shared_widget::CrossfadeIn;
use mpd_client::{
    client::{ConnectionEvent, Subsystem},
    commands::{CurrentSong, Next, Previous, SetPause, Stats, Status},
    responses::PlayState,
    Client,
};
use std::cell::Cell;
use std::io::{BufRead, BufReader};
use std::rc::Rc;
use tokio::net::TcpStream;

#[derive(Clone)]
struct PPButton {
    widget: Overlay,
    icon_revealer: Revealer,
    state_revealer: Revealer,
    state: Label,
}

impl PPButton {
    fn new() -> Self {
        let widget = Overlay::new();
        widget.add_css_class("icon-container");
        let icon_revealer = Revealer::builder()
            .transition_type(Crossfade)
            .child(&Label::new(Some("󰎆")))
            .reveal_child(true)
            .build();
        let state = Label::new(None);
        let state_revealer = Revealer::builder()
            .transition_type(Crossfade)
            .child(&state)
            .build();
        icon_revealer.add_css_class("icon");
        state_revealer.add_css_class("icon");
        let hover = EventControllerMotion::new();
        hover.connect_enter(clone! {
            #[strong] icon_revealer,
            #[strong] state_revealer,
            move |h, _, _| {
                icon_revealer.set_reveal_child(false);
                state_revealer.set_reveal_child(true);
            }
        });
        hover.connect_leave(clone! {
            #[strong] icon_revealer,
            #[strong] state_revealer,
            move |h| {
                icon_revealer.set_reveal_child(true);
                state_revealer.set_reveal_child(false);
            }
        });
        widget.add_controller(hover);
        widget.add_overlay(&icon_revealer);
        widget.add_overlay(&state_revealer);
        Self {
            widget,
            icon_revealer,
            state_revealer,
            state,
        }
    }
    fn set_state(&self, state: PlayState) {
        match state {
            PlayState::Stopped => self.state.set_text("󰐌"),
            PlayState::Playing => self.state.set_text("󰏥"),
            PlayState::Paused => self.state.set_text("󰐌"),
        }
    }
}

async fn left_ctl(mpd: &Client) -> (Box, PPButton) {
    let widget = Box::new(Horizontal, 5);
    let play_pause_button = PPButton::new();
    let prev_button = Revealer::builder()
        .transition_type(Crossfade)
        .child(&Label::new(Some("󰒮")))
        .build();
    let next_button = Revealer::builder()
        .transition_type(Crossfade)
        .child(&Label::new(Some("󰒭")))
        .build();
    prev_button.add_css_class("hidden");
    next_button.add_css_class("hidden");
    let hover = EventControllerMotion::new();
    let hover2 = EventControllerMotion::new();
    let prev = GestureClick::new();
    let next = GestureClick::new();
    let play_pause = GestureClick::new();

    if let Ok(status) = mpd.command(Status).await {
        // println!("{status:#?}");
        play_pause_button.set_state(status.state);
    }
    play_pause.connect_pressed(clone! {
        #[strong] mpd,
        move |_, _, _, _| {
            spawn_future_local(clone! {
                #[strong] mpd,
                async move {
                    if let Ok(status) = mpd.command(Status).await {
                        let paused = match status.state {
                            PlayState::Stopped => true,
                            PlayState::Playing => false,
                            PlayState::Paused => true,
                        };
                        mpd.command(SetPause(!paused)).await.unwrap();
                    }
                }
            });
        }
    });
    hover.connect_contains_pointer_notify(clone! {
        #[strong] prev_button,
        #[strong] next_button,
        move |h| {
            if h.contains_pointer() {
                prev_button.set_reveal_child(true);
                next_button.set_reveal_child(true);
            } else {
                prev_button.set_reveal_child(false);
                next_button.set_reveal_child(false);
            }
        }
    });
    hover2.connect_contains_pointer_notify(clone! {
        #[strong] prev_button,
        #[strong] next_button,
        move |h| {
            if h.contains_pointer() {
                prev_button.set_reveal_child(true);
                next_button.set_reveal_child(true);
            } else {
                prev_button.set_reveal_child(false);
                next_button.set_reveal_child(false);
            }
        }
    });
    prev.connect_pressed(clone! {
        #[strong] mpd,
        move |_, _, _, _| {tokio::spawn(clone! {
            #[strong] mpd,
            async move {
                mpd.command(Previous).await.unwrap();
            }
        });}
    });
    next.connect_pressed(clone! {
        #[strong] mpd,
        move |_, _, _, _| {tokio::spawn(clone! {
            #[strong] mpd,
            async move {
                mpd.command(Next).await.unwrap();
            }
        });}
    });
    play_pause_button.widget.add_controller(play_pause);
    prev_button.add_controller(prev);
    prev_button.add_controller(hover);
    next_button.add_controller(next);
    next_button.add_controller(hover2);
    widget.append(&prev_button);
    widget.append(&play_pause_button.widget);
    widget.append(&next_button);
    (widget, play_pause_button)
}

#[derive(Clone)]
struct Info {
    widget: Box,
    title: Label,
    artists: Label,
    title_crossfade_in: CrossfadeIn,
    artists_crossfade_in: CrossfadeIn,
    separator_crossade_in: CrossfadeIn,
}

impl Info {
    fn new() -> Self {
        let widget = Box::new(Horizontal, 10);
        let duration = Duration::from_millis(1500);
        let title = &Label::new(None);
        let artists = &Label::new(None);
        let title_crossfade_in = CrossfadeIn::new(title, duration);
        let artists_crossfade_in = CrossfadeIn::new(artists, duration);
        let separator_crossade_in = CrossfadeIn::new(&Label::new(Some("-")), duration);
        separator_crossade_in.widget.add_css_class("hidden");
        artists_crossfade_in.widget.add_css_class("hidden");
        let hover = EventControllerMotion::new();
        let timingout = Rc::new(Cell::new(false));
        hover.connect_contains_pointer_notify(clone! {
            #[strong(rename_to = sci)] separator_crossade_in,
            #[strong(rename_to = aci)] artists_crossfade_in,
            #[strong] artists,
            move |h| {
                if h.contains_pointer() {
                    if !artists.label().is_empty() {
                        sci.reveal(true);
                        aci.reveal(true);
                    }
                } else {
                    if !timingout.get() {
                        timingout.set(true);
                        spawn_future_local(clone! {
                            #[strong] h,
                            #[strong] sci,
                            #[strong] aci,
                            #[strong] timingout,
                            async move {
                                let mut timeout = 100;
                                while timeout > 0 && !h.contains_pointer() {
                                    sleep(Duration::from_millis(10)).await;
                                    timeout -= 1;
                                }
                                if timeout == 0 {
                                    sci.reveal(false);
                                    aci.reveal(false);
                                    timingout.set(false);
                                } else {
                                    timingout.set(false);
                                }
                            }
                        });
                    }
                }
            }
        });
        widget.add_controller(hover);
        widget.append(&title_crossfade_in.widget);
        widget.append(&separator_crossade_in.widget);
        widget.append(&artists_crossfade_in.widget);
        Self {
            widget,
            title: title.clone(),
            artists: artists.clone(),
            title_crossfade_in,
            artists_crossfade_in,
            separator_crossade_in,
        }
    }
    fn update(&self, title: Option<&str>, artists: Option<String>) {
        if let Some(title) = title {
            self.title.set_text(title);
            self.title_crossfade_in.reveal(true);
        } else {
            self.title_crossfade_in.reveal(false);
        }
        if let Some(artists) = artists {
            self.artists.set_text(&artists);
        } else {
            self.artists.set_text("");
        }
    }
}

// TODO :: add right next button
//      :: add drag to next & prev
//      :: add scroll to next & prev
pub fn new() -> Box {
    let widget = Box::new(Horizontal, 5);
    spawn_future_local(clone! {
        #[strong] widget,
        async move {
            let stream = TcpStream::connect("127.0.0.1:6600").await.unwrap();
            let (mpd, mut event) = Client::connect(stream).await.unwrap();
            let (left_ctl, ppbutton) = left_ctl(&mpd).await;
            let info = Info::new();
            let update =
                || clone! {
                    #[strong] mpd,
                    #[strong] ppbutton,
                    #[strong] info,
                    async move {
                        if let Ok(status) = mpd.command(Status).await {
                            ppbutton.set_state(status.state);
                            if !(status.state == PlayState::Stopped) {
                                if let Ok(Some(current)) = mpd.command(CurrentSong).await {
                                    info.update(current.song.title(), current.song.artists().get(0).cloned());
                                }
                            } else {
                                info.update(None, None);
                            }
                        }
                    }
                };
            update().await;

            widget.append(&left_ctl);
            widget.append(&info.widget);
            loop {
                if let Some(event) = event.next().await {
                    match event {
                        ConnectionEvent::SubsystemChange(Subsystem::Player) => {
                            update().await;
                        }
                        ConnectionEvent::ConnectionClosed(exit_code) => {eprintln!("{exit_code:#?}")}
                        e => {println!("{e:#?}")}
                    }
                }
            }
        }
    });
    widget
}
