//! Root Relm4 application: slim sidebar + routed main content.

use crate::db::Catalog;
use crate::models::{LibrarySection, NavItem, Route};
use crate::pages::{
    all_books::{AllBooksModel, AllBooksOut},
    book::{BookPageModel, BookPageOut},
    book_float::{BookFloatModel, BookFloatOut},
    home::{HomeOut, HomePageModel},
    library::{LibraryOut, LibraryPageModel},
    placeholder::PlaceholderPageModel,
    reader::{ReaderModel, ReaderOut},
    settings::SettingsPageModel,
    shelf_detail::{ShelfDetailModel, ShelfDetailOut},
    shelves_grid::{ShelvesGridModel, ShelvesOut},
};
use gtk::prelude::*;
use relm4::prelude::*;
use std::rc::Rc;

#[derive(Debug)]
pub enum AppMsg {
    Navigate(NavItem),
    Push(Route),
    Back,
    OpenBookDialog {
        book_id: i64,
    },
    /// Close float and open full book page in the main column.
    FloatOpenFull {
        book_id: i64,
    },
    CloseBookDialog,
    /// Open immersive reader for book_id.
    OpenReader {
        book_id: i64,
    },
}

enum PageSlot {
    Home(Controller<HomePageModel>),
    Library(Controller<LibraryPageModel>),
    AllBooks(Controller<AllBooksModel>),
    Shelves(Controller<ShelvesGridModel>),
    ShelfDetail(Controller<ShelfDetailModel>),
    Book(Controller<BookPageModel>),
    Reader(Controller<ReaderModel>),
    Settings(Controller<SettingsPageModel>),
    Placeholder(Controller<PlaceholderPageModel>),
    Widget(gtk::Box),
}

impl PageSlot {
    fn widget(&self) -> gtk::Widget {
        match self {
            PageSlot::Home(c) => c.widget().clone().upcast(),
            PageSlot::Library(c) => c.widget().clone().upcast(),
            PageSlot::AllBooks(c) => c.widget().clone().upcast(),
            PageSlot::Shelves(c) => c.widget().clone().upcast(),
            PageSlot::ShelfDetail(c) => c.widget().clone().upcast(),
            PageSlot::Book(c) => c.widget().clone().upcast(),
            PageSlot::Reader(c) => c.widget().clone().upcast(),
            PageSlot::Settings(c) => c.widget().clone().upcast(),
            PageSlot::Placeholder(c) => c.widget().clone().upcast(),
            PageSlot::Widget(b) => b.clone().upcast(),
        }
    }
}

struct FloatingBook {
    _controller: Controller<BookFloatModel>,
    window: gtk::Window,
}

pub struct AppModel {
    catalog: Rc<Catalog>,
    route: Route,
    history: Vec<Route>,
    sidebar_override: Option<NavItem>,
    page: Option<PageSlot>,
    floating: Option<FloatingBook>,
    /// Dynamic top-bar title override (book pages).
    title_override: Option<String>,
    subtitle_override: Option<String>,
}

impl AppModel {
    fn sidebar_item(&self) -> NavItem {
        self.sidebar_override
            .unwrap_or_else(|| self.route.sidebar_item())
    }

    fn top_title(&self) -> String {
        self.title_override
            .clone()
            .unwrap_or_else(|| self.route.title())
    }

    fn top_subtitle(&self) -> Option<String> {
        self.subtitle_override
            .clone()
            .or_else(|| self.route.subtitle())
    }

    fn build_page(
        catalog: &Rc<Catalog>,
        route: &Route,
        sender: &ComponentSender<Self>,
    ) -> PageSlot {
        match route {
            Route::Module(NavItem::Home) => {
                let ctrl = HomePageModel::builder().launch(catalog.clone()).forward(
                    sender.input_sender(),
                    |out| match out {
                        HomeOut::OpenBook { book_id } => AppMsg::Push(Route::BookPage { book_id }),
                        HomeOut::OpenBookDialog { book_id } => AppMsg::OpenBookDialog { book_id },
                    },
                );
                PageSlot::Home(ctrl)
            }
            Route::Module(NavItem::Library) => {
                let ctrl = LibraryPageModel::builder().launch(catalog.clone()).forward(
                    sender.input_sender(),
                    |out| match out {
                        LibraryOut::OpenSection(sec) => AppMsg::Push(Route::LibrarySection(sec)),
                    },
                );
                PageSlot::Library(ctrl)
            }
            Route::LibrarySection(LibrarySection::AllBooks) => {
                let ctrl = AllBooksModel::builder().launch(catalog.clone()).forward(
                    sender.input_sender(),
                    |out| match out {
                        AllBooksOut::OpenBook { book_id } => {
                            AppMsg::Push(Route::BookPage { book_id })
                        }
                        AllBooksOut::OpenBookDialog { book_id } => {
                            AppMsg::OpenBookDialog { book_id }
                        }
                    },
                );
                PageSlot::AllBooks(ctrl)
            }
            Route::LibrarySection(section) => PageSlot::Widget(placeholder_section(*section)),
            Route::Module(NavItem::Shelves) | Route::ShelvesGrid => {
                let ctrl =
                    ShelvesGridModel::builder()
                        .launch(())
                        .forward(sender.input_sender(), |out| match out {
                            ShelvesOut::OpenShelf { shelf_id } => {
                                AppMsg::Push(Route::ShelfDetail { shelf_id })
                            }
                        });
                PageSlot::Shelves(ctrl)
            }
            Route::ShelfDetail { shelf_id } => {
                let ctrl = ShelfDetailModel::builder().launch(*shelf_id).forward(
                    sender.input_sender(),
                    |out| match out {
                        ShelfDetailOut::OpenBook { book_id } => {
                            AppMsg::Push(Route::BookPage { book_id })
                        }
                        ShelfDetailOut::OpenBookDialog { book_id } => {
                            AppMsg::OpenBookDialog { book_id }
                        }
                    },
                );
                PageSlot::ShelfDetail(ctrl)
            }
            Route::BookPage { book_id } => {
                let id = *book_id;
                let ctrl = BookPageModel::builder()
                    .launch((catalog.clone(), id))
                    .forward(sender.input_sender(), move |out| match out {
                        BookPageOut::Back => AppMsg::Back,
                        BookPageOut::OpenReader => AppMsg::OpenReader { book_id: id },
                        BookPageOut::Deleted { .. } => AppMsg::Back,
                    });
                PageSlot::Book(ctrl)
            }
            Route::Reader { book_id } => {
                let id = *book_id;
                let ctrl = ReaderModel::builder()
                    .launch((catalog.clone(), id))
                    .forward(sender.input_sender(), |out| match out {
                        ReaderOut::Close => AppMsg::Back,
                    });
                PageSlot::Reader(ctrl)
            }
            Route::Module(NavItem::Settings) => {
                let ctrl = SettingsPageModel::builder().launch(()).detach();
                PageSlot::Settings(ctrl)
            }
            Route::Module(item) => {
                let ctrl = PlaceholderPageModel::builder().launch(*item).detach();
                PageSlot::Placeholder(ctrl)
            }
        }
    }

    fn swap_page(
        &mut self,
        content_host: &gtk::Box,
        route: Route,
        push_history: bool,
        sender: &ComponentSender<Self>,
    ) {
        if push_history {
            self.history.push(self.route.clone());
        } else {
            self.history.clear();
        }

        self.sidebar_override = match &route {
            Route::BookPage { .. } | Route::Reader { .. } => Some(self.sidebar_item()),
            _ => None,
        };

        self.title_override = None;
        self.subtitle_override = None;

        while let Some(child) = content_host.first_child() {
            content_host.remove(&child);
        }
        // Drop old controller only after unparenting — avoids
        // gtk_widget_is_ancestor criticals on disposed widgets.
        self.page = None;

        if let Route::BookPage { book_id } = &route {
            if let Ok(Some(b)) = self.catalog.get_book(*book_id) {
                self.title_override = Some(b.title.clone());
                self.subtitle_override = Some(b.authors_display().to_string());
            }
        }
        if let Route::Reader { book_id } = &route {
            if let Ok(Some(b)) = self.catalog.get_book(*book_id) {
                self.title_override = Some(b.title.clone());
                self.subtitle_override = Some("Reading".into());
            }
        }

        self.route = route;
        let page = Self::build_page(&self.catalog, &self.route, sender);
        content_host.append(&page.widget());
        self.page = Some(page);
    }
}

#[relm4::component(pub)]
impl Component for AppModel {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[root]
        main_window = gtk::Window {
            add_css_class: "kalam-window",
            set_title: Some("Kalam"),
            set_default_width: 1100,
            set_default_height: 720,

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_hexpand: true,
                set_vexpand: true,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    add_css_class: "kalam-sidebar",
                    set_hexpand: false,
                    set_vexpand: true,
                    #[watch]
                    set_visible: !model.route.is_reader(),

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        add_css_class: "kalam-sidebar-inner",
                        set_vexpand: true,

                        gtk::Label {
                            set_label: "KALAM",
                            add_css_class: "kalam-brand",
                            set_halign: gtk::Align::Center,
                        },

                        #[name = "top_nav"]
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 0,
                        },

                        gtk::Box {
                            set_vexpand: true,
                            add_css_class: "kalam-nav-spacer",
                        },

                        #[name = "bottom_nav"]
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 0,
                        },
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    add_css_class: "kalam-main",
                    set_hexpand: true,
                    set_vexpand: true,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        add_css_class: "kalam-topbar",
                        set_spacing: 12,
                        set_hexpand: true,
                        #[watch]
                        set_visible: !model.route.is_reader(),

                        #[name = "back_btn"]
                        gtk::Button {
                            set_label: "← Back",
                            add_css_class: "kalam-back-btn",
                            #[watch]
                            set_visible: !model.history.is_empty(),
                            connect_clicked => AppMsg::Back,
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_valign: gtk::Align::Center,
                            set_hexpand: true,

                            gtk::Label {
                                #[watch]
                                set_label: &model.top_title(),
                                add_css_class: "kalam-topbar-title",
                                set_halign: gtk::Align::Start,
                            },
                            gtk::Label {
                                #[watch]
                                set_label: model.top_subtitle().as_deref().unwrap_or(""),
                                #[watch]
                                set_visible: model.top_subtitle().is_some(),
                                add_css_class: "kalam-topbar-subtitle",
                                set_halign: gtk::Align::Start,
                            },
                        },
                    },

                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_hscrollbar_policy: gtk::PolicyType::Never,
                        #[watch]
                        set_vscrollbar_policy: if model.route.is_reader() {
                            gtk::PolicyType::Never
                        } else {
                            gtk::PolicyType::Automatic
                        },

                        #[name = "content_host"]
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            add_css_class: "kalam-content",
                            set_hexpand: true,
                            set_vexpand: true,
                        },
                    },
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let catalog = match Catalog::open() {
            Ok(c) => Rc::new(c),
            Err(err) => {
                eprintln!("kalam: failed to open catalog: {err}");
                // Last resort: still try — panic is worse UX
                Rc::new(Catalog::open().expect("catalog open"))
            }
        };

        let initial_route = Route::Module(NavItem::Home);
        let page = Self::build_page(&catalog, &initial_route, &sender);

        let model = AppModel {
            catalog,
            route: initial_route,
            history: Vec::new(),
            sidebar_override: None,
            page: Some(page),
            floating: None,
            title_override: None,
            subtitle_override: None,
        };

        let widgets = view_output!();

        for item in NavItem::ALL {
            let btn = make_nav_button(*item, *item == NavItem::Home);
            let item_copy = *item;
            let s = sender.clone();
            btn.connect_clicked(move |_| s.input(AppMsg::Navigate(item_copy)));
            btn.set_widget_name(&format!("nav-{:?}", item));
            if item.is_bottom() {
                widgets.bottom_nav.append(&btn);
            } else {
                widgets.top_nav.append(&btn);
            }
        }

        if let Some(page) = &model.page {
            widgets.content_host.append(&page.widget());
        }

        let _ = root;
        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            AppMsg::Navigate(item) => {
                let route = match item {
                    NavItem::Shelves => Route::ShelvesGrid,
                    other => Route::Module(other),
                };
                if let Some(f) = self.floating.take() {
                    f.window.set_child(None::<&gtk::Widget>);
                    f.window.destroy();
                }
                self.swap_page(&widgets.content_host, route, false, &sender);
            }
            AppMsg::Push(route) => {
                self.swap_page(&widgets.content_host, route, true, &sender);
            }
            AppMsg::Back => {
                if let Some(prev) = self.history.pop() {
                    // Like swap_page but keep remaining history.
                    self.sidebar_override = match &prev {
                        Route::BookPage { .. } => self.sidebar_override,
                        _ => None,
                    };
                    self.title_override = None;
                    self.subtitle_override = None;

                    while let Some(child) = widgets.content_host.first_child() {
                        widgets.content_host.remove(&child);
                    }
                    self.page = None;

                    if let Route::BookPage { book_id } = &prev {
                        if let Ok(Some(b)) = self.catalog.get_book(*book_id) {
                            self.title_override = Some(b.title.clone());
                            self.subtitle_override = Some(b.authors_display().to_string());
                        }
                    }

                    self.route = prev;
                    let page = Self::build_page(&self.catalog, &self.route, &sender);
                    widgets.content_host.append(&page.widget());
                    self.page = Some(page);
                }
            }
            AppMsg::OpenBookDialog { book_id } => {
                if let Some(f) = self.floating.take() {
                    f.window.set_child(None::<&gtk::Widget>);
                    f.window.destroy();
                }

                let ctrl = BookFloatModel::builder()
                    .launch((self.catalog.clone(), book_id))
                    .forward(sender.input_sender(), |out| match out {
                        BookFloatOut::Close | BookFloatOut::Deleted { .. } => {
                            AppMsg::CloseBookDialog
                        }
                        BookFloatOut::OpenReader { book_id } => AppMsg::OpenReader { book_id },
                        BookFloatOut::OpenFullPage { book_id } => AppMsg::FloatOpenFull { book_id },
                    });

                let title = self
                    .catalog
                    .get_book(book_id)
                    .ok()
                    .flatten()
                    .map(|b| b.title)
                    .unwrap_or_else(|| "Book".into());

                // No open/close animation for now — plain panel.
                let window = gtk::Window::builder()
                    .title(title)
                    .transient_for(root)
                    .default_width(900)
                    .default_height(560)
                    .resizable(true)
                    .modal(false)
                    .decorated(false)
                    .build();
                window.add_css_class("kalam-window");
                window.add_css_class("kalam-float-window");
                window.set_child(Some(ctrl.widget()));

                let key = gtk::EventControllerKey::new();
                let s_key = sender.clone();
                key.connect_key_pressed(move |_, keyval, _, _| {
                    use gtk::gdk::Key;
                    if keyval == Key::q || keyval == Key::Q || keyval == Key::Escape {
                        s_key.input(AppMsg::CloseBookDialog);
                        return gtk::glib::Propagation::Stop;
                    }
                    gtk::glib::Propagation::Proceed
                });
                window.add_controller(key);

                let s = sender.clone();
                window.connect_close_request(move |_| {
                    s.input(AppMsg::CloseBookDialog);
                    gtk::glib::Propagation::Stop
                });

                window.present();
                ctrl.widget().grab_focus();

                self.floating = Some(FloatingBook {
                    _controller: ctrl,
                    window,
                });
            }
            AppMsg::FloatOpenFull { book_id } => {
                if let Some(f) = self.floating.take() {
                    f.window.set_child(None::<&gtk::Widget>);
                    f.window.destroy();
                }
                self.swap_page(
                    &widgets.content_host,
                    Route::BookPage { book_id },
                    true,
                    &sender,
                );
            }
            AppMsg::CloseBookDialog => {
                if let Some(f) = self.floating.take() {
                    f.window.set_child(None::<&gtk::Widget>);
                    f.window.destroy();
                }
            }
            AppMsg::OpenReader { book_id } => {
                if let Some(f) = self.floating.take() {
                    f.window.set_child(None::<&gtk::Widget>);
                    f.window.destroy();
                }
                self.swap_page(
                    &widgets.content_host,
                    Route::Reader { book_id },
                    true,
                    &sender,
                );
            }
        }

        let active = self.sidebar_item();
        update_nav_styles(&widgets.top_nav, active);
        update_nav_styles(&widgets.bottom_nav, active);
        self.update_view(widgets, sender);
    }
}

fn make_nav_button(item: NavItem, active: bool) -> gtk::Button {
    let inner = gtk::Box::new(gtk::Orientation::Vertical, 2);
    inner.set_halign(gtk::Align::Center);

    let icon = gtk::Label::new(Some(item.icon()));
    icon.add_css_class("kalam-nav-icon");
    icon.set_halign(gtk::Align::Center);

    let label = gtk::Label::new(Some(item.label()));
    label.set_halign(gtk::Align::Center);

    inner.append(&icon);
    inner.append(&label);

    let btn = gtk::Button::new();
    btn.set_child(Some(&inner));
    btn.add_css_class("kalam-nav-btn");
    if active {
        btn.add_css_class("active");
    }
    btn.set_tooltip_text(Some(item.label()));
    btn.set_focus_on_click(false);
    btn
}

fn update_nav_styles(container: &gtk::Box, active: NavItem) {
    let mut child = container.first_child();
    while let Some(widget) = child {
        if let Ok(btn) = widget.clone().downcast::<gtk::Button>() {
            let name = btn.widget_name();
            if name == format!("nav-{:?}", active) {
                btn.add_css_class("active");
            } else {
                btn.remove_css_class("active");
            }
        }
        child = widget.next_sibling();
    }
}

fn placeholder_section(section: LibrarySection) -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    let title = gtk::Label::new(Some(section.label()));
    title.add_css_class("kalam-page-title");
    title.set_halign(gtk::Align::Start);
    page.append(&title);

    let sub = gtk::Label::new(Some(section.blurb()));
    sub.add_css_class("kalam-page-sub");
    sub.set_halign(gtk::Align::Start);
    page.append(&sub);

    let ph = gtk::Label::new(Some(&format!(
        "{} — coming in a later phase. Use All books for your catalog.",
        section.label()
    )));
    ph.add_css_class("kalam-placeholder");
    ph.set_wrap(true);
    page.append(&ph);
    page
}
