//! Root Relm4 application: slim sidebar + routed main content.

use crate::db::Catalog;
use crate::models::{LibrarySection, NavItem, Route};
use crate::pages::{
    all_books::{AllBooksModel, AllBooksOut},
    analytics::AnalyticsModel,
    book::{BookPageModel, BookPageOut},
    book_float::{BookFloatModel, BookFloatOut},
    history::{HistoryModel, HistoryOut},
    home::{HomeOut, HomePageModel},
    library::{LibraryOut, LibraryPageModel},
    placeholder::PlaceholderPageModel,
    reader::{ReaderModel, ReaderOut},
    reading_list::{ReadingListModel, ReadingListOut},
    saved_quotes::{SavedQuotesModel, SavedQuotesOut},
    saved_words::{SavedWordsModel, SavedWordsOut},
    settings::SettingsPageModel,
    shelf_detail::{ShelfDetailModel, ShelfDetailOut},
    shelves_grid::{ShelvesGridModel, ShelvesOut},
    tags::{TagBooksModel, TagBooksOut, TagsModel, TagsOut},
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
    /// Rebuild the page on screen if the catalog changed under it.
    RefreshCurrentPage,
    /// Open immersive reader for book_id.
    OpenReader {
        book_id: i64,
    },
}

enum PageSlot {
    Home(Controller<HomePageModel>),
    Library(Controller<LibraryPageModel>),
    AllBooks(Controller<AllBooksModel>),
    SavedQuotes(Controller<SavedQuotesModel>),
    SavedWords(Controller<SavedWordsModel>),
    Shelves(Controller<ShelvesGridModel>),
    ShelfDetail(Controller<ShelfDetailModel>),
    ReadingList(Controller<ReadingListModel>),
    History(Controller<HistoryModel>),
    Tags(Controller<TagsModel>),
    TagBooks(Controller<TagBooksModel>),
    Analytics(Controller<AnalyticsModel>),
    Book(Controller<BookPageModel>),
    Reader(Controller<ReaderModel>),
    Settings(Controller<SettingsPageModel>),
    Placeholder(Controller<PlaceholderPageModel>),
}

impl PageSlot {
    fn widget(&self) -> gtk::Widget {
        match self {
            PageSlot::Home(c) => c.widget().clone().upcast(),
            PageSlot::Library(c) => c.widget().clone().upcast(),
            PageSlot::AllBooks(c) => c.widget().clone().upcast(),
            PageSlot::SavedQuotes(c) => c.widget().clone().upcast(),
            PageSlot::SavedWords(c) => c.widget().clone().upcast(),
            PageSlot::Shelves(c) => c.widget().clone().upcast(),
            PageSlot::ShelfDetail(c) => c.widget().clone().upcast(),
            PageSlot::ReadingList(c) => c.widget().clone().upcast(),
            PageSlot::History(c) => c.widget().clone().upcast(),
            PageSlot::Tags(c) => c.widget().clone().upcast(),
            PageSlot::TagBooks(c) => c.widget().clone().upcast(),
            PageSlot::Analytics(c) => c.widget().clone().upcast(),
            PageSlot::Book(c) => c.widget().clone().upcast(),
            PageSlot::Reader(c) => c.widget().clone().upcast(),
            PageSlot::Settings(c) => c.widget().clone().upcast(),
            PageSlot::Placeholder(c) => c.widget().clone().upcast(),
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
    title_override: Option<String>,
    subtitle_override: Option<String>,
    /// Pages kept alive between visits, keyed by route.
    ///
    /// Rebuilding a whole widget tree on every click was the second half of
    /// the UI lag: revisiting Home or Library reconstructed dozens of widgets
    /// and re-ran their queries. Cached pages are unparented rather than
    /// destroyed, so returning to one costs nothing.
    cache: Vec<(String, PageSlot)>,
    /// Catalog write counter at the time each cached page was built. A cached
    /// page is only reused while this matches, so an import, delete or edit
    /// anywhere automatically forces a rebuild — no write path has to
    /// remember to invalidate.
    cache_token: i64,
}

/// Cache key for a route, or `None` for pages that must always be rebuilt.
///
/// Reader is excluded deliberately: it owns a WebView and a reading session,
/// and must be torn down on leave. Book and shelf pages are excluded because
/// their content changes as you edit metadata, ratings and membership.
fn cache_key(route: &Route) -> Option<String> {
    match route {
        Route::Module(NavItem::Home) => Some("home".into()),
        Route::Module(NavItem::Library) => Some("library".into()),
        Route::Module(NavItem::Shelves) | Route::ShelvesGrid => Some("shelves".into()),
        Route::Module(NavItem::Settings) => Some("settings".into()),
        Route::Module(item) => Some(format!("mod:{}", item.label())),
        // Everything below reflects data the user is actively changing.
        Route::LibrarySection(_)
        | Route::ShelfDetail { .. }
        | Route::TagBooks { .. }
        | Route::BookPage { .. }
        | Route::Reader { .. } => None,
    }
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
                        LibraryOut::Section(sec) => AppMsg::Push(Route::LibrarySection(sec)),
                        LibraryOut::Book { book_id } => AppMsg::Push(Route::BookPage { book_id }),
                        LibraryOut::BookDialog { book_id } => AppMsg::OpenBookDialog { book_id },
                        LibraryOut::Tag { tag } => AppMsg::Push(Route::TagBooks { tag }),
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
            Route::LibrarySection(LibrarySection::SavedQuotes) => {
                let cat = catalog.clone();
                let ctrl =
                    SavedQuotesModel::builder()
                        .launch(cat)
                        .forward(sender.input_sender(), |out| match out {
                            SavedQuotesOut::OpenBook { book_id } => {
                                AppMsg::Push(Route::BookPage { book_id })
                            }
                            SavedQuotesOut::JumpTo { book_id, .. } => {
                                AppMsg::Push(Route::BookPage { book_id })
                            }
                        });
                PageSlot::SavedQuotes(ctrl)
            }
            Route::LibrarySection(LibrarySection::SavedWords) => {
                let cat = catalog.clone();
                let ctrl =
                    SavedWordsModel::builder()
                        .launch(cat)
                        .forward(sender.input_sender(), |out| match out {
                            SavedWordsOut::OpenBook { book_id } => {
                                AppMsg::Push(Route::BookPage { book_id })
                            }
                        });
                PageSlot::SavedWords(ctrl)
            }
            Route::LibrarySection(LibrarySection::ReadingList) => {
                let ctrl = ReadingListModel::builder().launch(catalog.clone()).forward(
                    sender.input_sender(),
                    |out| match out {
                        ReadingListOut::OpenBook { book_id } => {
                            AppMsg::Push(Route::BookPage { book_id })
                        }
                        ReadingListOut::OpenReader { book_id } => AppMsg::OpenReader { book_id },
                    },
                );
                PageSlot::ReadingList(ctrl)
            }
            Route::LibrarySection(LibrarySection::History) => {
                let ctrl = HistoryModel::builder().launch(catalog.clone()).forward(
                    sender.input_sender(),
                    |out| match out {
                        HistoryOut::OpenBook { book_id } => {
                            AppMsg::Push(Route::BookPage { book_id })
                        }
                    },
                );
                PageSlot::History(ctrl)
            }
            Route::LibrarySection(LibrarySection::Tags) => {
                let ctrl = TagsModel::builder().launch(catalog.clone()).forward(
                    sender.input_sender(),
                    |out| match out {
                        TagsOut::OpenTag { tag } => AppMsg::Push(Route::TagBooks { tag }),
                    },
                );
                PageSlot::Tags(ctrl)
            }
            Route::LibrarySection(LibrarySection::Analytics) => {
                let ctrl = AnalyticsModel::builder().launch(catalog.clone()).detach();
                PageSlot::Analytics(ctrl)
            }
            Route::Module(NavItem::Shelves) | Route::ShelvesGrid => {
                let ctrl = ShelvesGridModel::builder().launch(catalog.clone()).forward(
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
                    .launch((catalog.clone(), *shelf_id))
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
            Route::TagBooks { tag } => {
                let ctrl = TagBooksModel::builder()
                    .launch((catalog.clone(), tag.clone()))
                    .forward(sender.input_sender(), |out| match out {
                        TagBooksOut::OpenBook { book_id } => {
                            AppMsg::Push(Route::BookPage { book_id })
                        }
                        TagBooksOut::OpenBookDialog { book_id } => {
                            AppMsg::OpenBookDialog { book_id }
                        }
                    });
                PageSlot::TagBooks(ctrl)
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
                let ctrl = SettingsPageModel::builder()
                    .launch(catalog.clone())
                    .detach();
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

        self.detach_current(content_host);

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
        if let Route::ShelfDetail { shelf_id } = &route {
            if let Ok(Some(shelf)) = self.catalog.get_shelf(*shelf_id) {
                self.title_override = Some(shelf.name.clone());
                self.subtitle_override = Some(shelf.summary());
            }
        }
        if let Route::TagBooks { tag } = &route {
            self.title_override = Some(format!("#{tag}"));
        }

        // Toggle reader padding class without negative margins.
        if route.is_reader() {
            content_host.add_css_class("kalam-content-reader");
        } else {
            content_host.remove_css_class("kalam-content-reader");
        }

        self.route = route;
        let page = self.take_or_build(sender);
        content_host.append(&page.widget());
        self.page = Some(page);
    }

    /// Unparent the current page, parking it in the cache when its route is
    /// cacheable and dropping it otherwise.
    fn detach_current(&mut self, content_host: &gtk::Box) {
        while let Some(child) = content_host.first_child() {
            content_host.remove(&child);
        }

        // Only drop the controller *after* unparenting — otherwise GTK probes
        // a disposed widget and logs gtk_widget_is_ancestor criticals.
        let Some(page) = self.page.take() else {
            return;
        };
        if let Some(key) = cache_key(&self.route) {
            if !self.cache.iter().any(|(k, _)| *k == key) {
                self.cache.push((key, page));
            }
        }
    }

    /// Rebuild the current page if the catalog changed since it was built.
    ///
    /// Deleting a book only updated the database; whatever page was on screen
    /// kept its stale widgets until the next navigation, so a removed book
    /// lingered on Home until you switched tabs.
    fn refresh_if_stale(&mut self, content_host: &gtk::Box, sender: &ComponentSender<Self>) {
        let token = self.catalog.change_token();
        if token == self.cache_token {
            return;
        }

        // Unparent before dropping, as everywhere else, or GTK complains about
        // a disposed widget. Nothing is cached here: every cached page was
        // built against the old catalog state.
        while let Some(child) = content_host.first_child() {
            content_host.remove(&child);
        }
        self.page = None;
        self.cache.clear();
        self.cache_token = token;

        // Rebuild the same route in place — no history push.
        let page = Self::build_page(&self.catalog, &self.route, sender);
        content_host.append(&page.widget());
        self.page = Some(page);
    }

    /// Reuse a cached page for the current route, or build a fresh one.
    fn take_or_build(&mut self, sender: &ComponentSender<Self>) -> PageSlot {
        // Any catalog write invalidates every cached page: a stale Home would
        // happily show a book you just deleted.
        let token = self.catalog.change_token();
        if token != self.cache_token {
            self.cache.clear();
            self.cache_token = token;
        }

        if let Some(key) = cache_key(&self.route) {
            if let Some(idx) = self.cache.iter().position(|(k, _)| *k == key) {
                let (_, page) = self.cache.remove(idx);
                return page;
            }
        }
        Self::build_page(&self.catalog, &self.route, sender)
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
                Rc::new(Catalog::open().expect("catalog open"))
            }
        };

        let initial_route = Route::Module(NavItem::Home);
        let page = Self::build_page(&catalog, &initial_route, &sender);

        let cache_token = catalog.change_token();
        let model = AppModel {
            catalog,
            route: initial_route,
            history: Vec::new(),
            sidebar_override: None,
            page: Some(page),
            floating: None,
            title_override: None,
            subtitle_override: None,
            cache: Vec::new(),
            cache_token,
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
                    self.sidebar_override = match &prev {
                        Route::BookPage { .. } => self.sidebar_override,
                        _ => None,
                    };
                    self.title_override = None;
                    self.subtitle_override = None;

                    self.detach_current(&widgets.content_host);

                    if let Route::BookPage { book_id } = &prev {
                        if let Ok(Some(b)) = self.catalog.get_book(*book_id) {
                            self.title_override = Some(b.title.clone());
                            self.subtitle_override = Some(b.authors_display().to_string());
                        }
                    }

                    // Fix crooked home after reader: ensure reader class is removed when leaving reader
                    if prev.is_reader() {
                        widgets.content_host.add_css_class("kalam-content-reader");
                    } else {
                        widgets
                            .content_host
                            .remove_css_class("kalam-content-reader");
                    }

                    self.route = prev;
                    let page = self.take_or_build(&sender);
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
                // The float can delete a book, so the page underneath may now
                // be showing something that no longer exists. Deferred to the
                // next main-loop turn: rebuilding here would dispose widgets
                // while GTK is still unwinding the float's close signal, which
                // is what produced the gtk_widget_is_ancestor criticals.
                let s = sender.clone();
                gtk::glib::idle_add_local_once(move || {
                    s.input(AppMsg::RefreshCurrentPage);
                });
            }
            AppMsg::RefreshCurrentPage => {
                self.refresh_if_stale(&widgets.content_host, &sender);
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
    btn.set_halign(gtk::Align::Fill);
    btn.set_hexpand(true);
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
