use crate::*;
use gtk::PopoverMenu;

pub fn new() -> Box {
    let widget = Box::default();
    let pop_button = Button::with_label("Pop");
    widget.append(&pop_button);
    let popup = PopoverMenu::builder().has_arrow(false).build();
    popup.set_parent(&pop_button);
    let stuff = Label::new(Some("Hello, World!"));
    popup.set_child(Some(&stuff));
    pop_button.connect_clicked(clone!(
        #[strong]
        popup,
        move |_button| {
            popup.popup();
        }
    ));
    popup.connect_closed(|_| {
        println!("autohide");
    });
    widget
}
