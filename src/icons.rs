use gtk::prelude::*;

/// One symbolic icon from the current icon theme.
pub fn symbolic(name: &str, pixel_size: i32) -> gtk::Image {
    let image = gtk::Image::builder().icon_name(name).build();
    image.set_pixel_size(pixel_size);
    image
}

/// One symbolic icon with CSS classes attached.
pub fn symbolic_with_classes(name: &str, pixel_size: i32, classes: &[&str]) -> gtk::Image {
    let image = symbolic(name, pixel_size);
    for class in classes {
        image.add_css_class(class);
    }
    image
}

/// A small horizontal row: icon then plain label.
pub fn labelled(icon_name: &str, pixel_size: i32, label: &str, spacing: i32) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, spacing);
    row.append(&symbolic_with_classes(
        icon_name,
        pixel_size,
        &["kalam-inline-icon"],
    ));
    row.append(&gtk::Label::new(Some(label)));
    row
}
