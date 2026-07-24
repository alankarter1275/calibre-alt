//! Root Relm4 application: slim sidebar + routed main content.

use crate::models::{book_by_id, sample_books, LibrarySection, NavItem, Route};
use crate::pages::{
    book::{BookPageModel, BookPageOut},
    home::{HomeOut, HomePageModel},
    library::{LibraryOut, LibraryPageModel},
    placeholder::PlaceholderPageModel,
    shelf_detail::{ShelfDetailModel, ShelfDetailOut},
    shelves_grid::{ShelvesGridModel, ShelvesOut},
};
use crate::widgets::book_row::build_book_row;
use gtk::prelude::*;
use relm4::prelude::*;

#[derive(Debug)]
pub enum AppMsg {
    /// Sidebar button clicked — replaces the stack.
    Navigate(NavItem),
    /// Push a deeper route (shelf, book, library section…).
    Push(Route),
    /// Pop one level of the in-module stack.
    Back,
    /// Open book as a floating window.
    OpenBookDialog { book_id: u64 },
    /// Floating window closed.
    CloseBookDialog,
}

/// Active page controller kept alive while its route is shown.
enum PageSlot {
    Home(Controller<HomePageModel>),
    Library(Controller<LibraryPageModel>),
    Shelves(Controller<ShelvesGridModel>),
    ShelfDetail(Controller<ShelfDetailModel>),
    Book(Controller<BookPageModel>),
    Placeholder(Controller<PlaceholderPageModel>),
    /// Plain GTK widget tree (library sections).
    Widget(gtk::Box),
}

impl PageSlot {
    fn widget(&self) -> gtk::Widget {
        match self {
            PageSlot::Home(c) => c.widget().clone().upcast(),
            PageSlot::Library(c) => c.widget().clone().upcast(),
            PageSlot::Shelves(c) => c.widget().clone().upcast(),
            PageSlot::ShelfDetail(c) => c.widget().clone().upcast(),
            PageSlot::Book(c) => c.widget().clone().upcast(),
            PageSlot::Placeholder(c) => c.widget().clone().upcast(),
            PageSlot::Widget(b) => b.clone().upcast(),
        }
    }
}

struct FloatingBook {
    _controller: Controller<BookPageModel>,
    window: gtk::Window,
}

pub struct AppModel {
    route: Route,
    /// Stack of routes for Back (does not include current).
    history: Vec<Route>,
    /// Sidebar highlight while a book page is open.
    sidebar_override: Option<NavItem>,
    page: Option<PageSlot>,
    floating: Option<FloatingBook>,
}

impl AppModel {
    fn sidebar_item(&self) -> NavItem {
        self.sidebar_override
            .unwrap_or_else(|| self.route.sidebar_item())
    }

    fn build_page(route: &Route, sender: &ComponentSender<Self>) -> PageSlot {
        match route {
            Route::Module(NavItem::Home) => {
                let ctrl = HomePageModel::builder().launch(()).forward(
                    sender.input_sender(),
                    |out| match out {
                        HomeOut::OpenBook { book_id } => AppMsg::Push(Route::BookPage { book_id }),
                        HomeOut::OpenBookDialog { book_id } => AppMsg::OpenBookDialog { book_id },
                    },
                );
                PageSlot::Home(ctrl)
            }
            Route::Module(NavItem::Library) => {
                let ctrl = LibraryPageModel::builder().launch(()).forward(
                    sender.input_sender(),
                    |out| match out {
                        LibraryOut::OpenSection(sec) => AppMsg::Push(Route::LibrarySection(sec)),
                    },
                );
                PageSlot::Library(ctrl)
            }
            Route::LibrarySection(section) => {
                PageSlot::Widget(build_library_section_page(*section, sender.clone()))
            }
            Route::Module(NavItem::Shelves) | Route::ShelvesGrid => {
                let ctrl = ShelvesGridModel::builder().launch(()).forward(
                    sender.input_sender(),
                    |out| match out {
                        ShelvesOut::OpenShelf { shelf_id } => {
                            AppMsg::Push(Route::ShelfDetail { shelf_id })
                        }
                    },
                );
                PageSlot::Shelves(ctrl)
            }
            Route::ShelfDetail { shelf_id } => {
                let ctrl = ShelfDetailModel::builder()
                    .launch(*shelf_id)
                    .forward(sender.input_sender(), |out| match out {
                        ShelfDetailOut::OpenBook { book_id } => {
                            AppMsg::Push(Route::BookPage { book_id })
                        }
                        ShelfDetailOut::OpenBookDialog { book_id } => {
                            AppMsg::OpenBookDialog { book_id }
                        }
                    });
                PageSlot::ShelfDetail(ctrl)
            }
            Route::BookPage { book_id } => {
                let ctrl = BookPageModel::builder()
                    .launch(*book_id)
                    .forward(sender.input_sender(), |out| match out {
                        BookPageOut::Back => AppMsg::Back,
                        // Reader is P2 — keep the user on the book page.
                        BookPageOut::OpenReader => AppMsg::CloseBookDialog,
                    });
                PageSlot::Book(ctrl)
            }
            Route::Module(item) => {
                let ctrl = PlaceholderPageModel::builder().launch(*item).detach();
                PageSlot::Placeholder(ctrl)
            }
        }
    }

    /// Unparent the current page, then install `route`.
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
            Route::BookPage { .. } => Some(self.sidebar_item()),
            _ => None,
        };

        // Unparent before dropping the old controller.
        while let Some(child) = content_host.first_child() {
            content_host.remove(&child);
        }
        self.page = None;

        self.route = route;
        let page = Self::build_page(&self.route, sender);
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
                                set_label: &model.route.title(),
                                add_css_class: "kalam-topbar-title",
                                set_halign: gtk::Align::Start,
                            },
                            gtk::Label {
                                #[watch]
                                set_label: model.route.subtitle().as_deref().unwrap_or(""),
                                #[watch]
                                set_visible: model.route.subtitle().is_some(),
                                add_css_class: "kalam-topbar-subtitle",
                                set_halign: gtk::Align::Start,
                            },
                        },
                    },

                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_hscrollbar_policy: gtk::PolicyType::Never,

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
        let initial_route = Route::Module(NavItem::Home);
        let page = Self::build_page(&initial_route, &sender);

        let model = AppModel {
            route: initial_route,
            history: Vec::new(),
            sidebar_override: None,
            page: Some(page),
            floating: None,
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
                    f.window.close();
                }
                self.swap_page(&widgets.content_host, route, false, &sender);
            }
            AppMsg::Push(route) => {
                self.swap_page(&widgets.content_host, route, true, &sender);
            }
            AppMsg::Back => {
                if let Some(prev) = self.history.pop() {
                    self.sidebar_override = None;
                    while let Some(child) = widgets.content_host.first_child() {
                        widgets.content_host.remove(&child);
                    }
                    self.page = None;
                    self.route = prev;
                    let page = Self::build_page(&self.route, &sender);
                    widgets.content_host.append(&page.widget());
                    self.page = Some(page);
                }
            }
            AppMsg::OpenBookDialog { book_id } => {
                if let Some(f) = self.floating.take() {
                    f.window.close();
                }

                let ctrl = BookPageModel::builder()
                    .launch(book_id)
                    .forward(sender.input_sender(), |out| match out {
                        BookPageOut::Back | BookPageOut::OpenReader => AppMsg::CloseBookDialog,
                    });

                let title = book_by_id(book_id)
                    .map(|b| b.title.clone())
                    .unwrap_or_else(|| "Book".into());

                let window = gtk::Window::builder()
                    .title(title)
                    .transient_for(root)
                    .default_width(560)
                    .default_height(680)
                    .modal(false)
                    .build();
                window.add_css_class("kalam-window");
                window.set_child(Some(ctrl.widget()));

                let s = sender.clone();
                window.connect_destroy(move |_| {
                    s.input(AppMsg::CloseBookDialog);
                });

                window.present();
                self.floating = Some(FloatingBook {
                    _controller: ctrl,
                    window,
                });
            }
            AppMsg::CloseBookDialog => {
                if let Some(f) = self.floating.take() {
                    f.window.close();
                }
            }
        }

        // Sidebar active state + #[watch] fields
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

fn build_library_section_page(
    section: LibrarySection,
    sender: ComponentSender<AppModel>,
) -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 12);

    let title = gtk::Label::new(Some(section.label()));
    title.add_css_class("kalam-page-title");
    title.set_halign(gtk::Align::Start);
    page.append(&title);

    let sub = gtk::Label::new(Some(section.blurb()));
    sub.add_css_class("kalam-page-sub");
    sub.set_halign(gtk::Align::Start);
    page.append(&sub);

    match section {
        LibrarySection::AllBooks | LibrarySection::ReadingList | LibrarySection::History => {
            for book in sample_books() {
                let id = book.id;
                let s1 = sender.clone();
                let s2 = sender.clone();
                let row = build_book_row(
                    book,
                    move || s1.input(AppMsg::Push(Route::BookPage { book_id: id })),
                    move || s2.input(AppMsg::OpenBookDialog { book_id: id }),
                );
                page.append(&row);
            }
        }
        other => {
            let ph = gtk::Label::new(Some(&format!(
                "{} — placeholder. Real data arrives in a later phase.",
                other.label()
            )));
            ph.add_css_class("kalam-placeholder");
            ph.set_wrap(true);
            page.append(&ph);
        }
    }

    page
}
