use crate::*;
use alsa::{card::Card, ctl::Ctl};
use async_std::channel::unbounded;
use pulsectl::controllers::SinkController;

pub fn new() -> Box {
    let widget = Box::default();
    widget.add_css_class("container");
    widget.add_css_class("volume");
    let card = Card::new(1);
    let name = card.get_name().unwrap();
    println!("{name}");
    let ctl = Ctl::from_card(&card, false).unwrap();
    ctl.subscribe_events(true).unwrap();
    let (tx, rx) = unbounded::<i32>();
    tokio::task::spawn_blocking(move || {
        let mut i = 0;
        loop {
            if let Ok(Some(res)) = ctl.read() {
                if res.get_mask().value() {
                    tx.send_blocking(-2).unwrap();
                }
                tx.send_blocking(i).unwrap();
                i += 1;
            } else {
                tx.send_blocking(-1).unwrap();
            }
        }
    });
    spawn_future(async move {
        loop {
            if let Ok(res) = rx.recv().await {
                println!("{res:?}");
            }
        }
    });
    widget
}
