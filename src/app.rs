//! Root Relm4 application: slim sidebar + routed main content.

use crate::db::Catalog;
use crate::models::{LibrarySection, NavItem, Route};
use crate::pages::{
    all_books::{AllBooksModel, AllBooksOut},
    analytics::AnalyticsModel,
    author::{AuthorPageModel, AuthorPageOut},
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
    series_float::{SeriesFloatModel, SeriesFloatOut},
    settings::SettingsPageModel,
    shelf_detail::{ShelfDetailModel, ShelfDetailOut},
    shelves_grid::{ShelvesGridModel, ShelvesOut},
    tags::{TagBooksModel, TagBooksOut, TagsModel, TagsOut},
};
use gtk::prelude::*;
use relm4::prelude::*;
use std::sync::Arc;

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
    /// Open the series float from the book page's Series link.
    OpenSeriesFloat {
        series: String,
        first_author: String,
    },
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
    Author(Controller<AuthorPageModel>),
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
            PageSlot::Author(c) => c.widget().clone().upcast(),
            PageSlot::Book(c) => c.widget().clone().upcast(),
            PageSlot::Reader(c) => c.widget().clone().upcast(),
            PageSlot::Settings(c) => c.widget().clone().upcast(),
            PageSlot::Placeholder(c) => c.widget().clone().upcast(),
        }
    }
}

/// Which float is on screen — only one at a time.
enum Floating {
    Book(Controller<BookFloatModel>),
    Series(Controller<SeriesFloatModel>),
}

pub struct AppModel {
    catalog: Arc<Catalog>,
    route: Route,
    history: Vec<Route>,
    sidebar_override: Option<NavItem>,
    page: Option<PageSlot>,
    floating: Option<Floating>,
    float_scrim: gtk::Box,
    float_host: gtk::Box,
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
        | Route::AuthorPage { .. }
        | Route::BookPage { .. }
        | Route::Reader { .. } => None,
    }
}

impl AppModel {
    fn sidebar_item(&self) -> NavItem {
        self.sidebar_override
            .unwrap_or_else(|| self.route.sidebar_item())
    }

    fn show_back_chip(&self) -> bool {
        !self.history.is_empty() && !self.route.is_reader() && !self.route.is_book_page()
    }

    fn close_floating(&mut self) {
        while let Some(child) = self.float_host.first_child() {
            self.float_host.remove(&child);
        }
        self.float_host.set_visible(false);
        self.float_scrim.set_visible(false);
        self.floating = None;
    }

    fn open_floating(&mut self, book_id: i64, sender: &ComponentSender<Self>) {
        self.close_floating();

        let ctrl = BookFloatModel::builder()
            .launch((self.catalog.clone(), book_id))
            .forward(sender.input_sender(), |out| match out {
                BookFloatOut::Close | BookFloatOut::Deleted { .. } => AppMsg::CloseBookDialog,
                BookFloatOut::OpenReader { book_id } => AppMsg::OpenReader { book_id },
                BookFloatOut::OpenFullPage { book_id } => AppMsg::FloatOpenFull { book_id },
                BookFloatOut::OpenAuthor { name } => {
                    AppMsg::Push(Route::AuthorPage { author: name })
                }
            });

        let float = ctrl.widget().clone();
        float.set_size_request(720, 420);
        float.set_hexpand(false);
        float.set_vexpand(false);
        float.set_halign(gtk::Align::Center);
        float.set_valign(gtk::Align::Center);
        self.float_host.append(&float);
        self.float_scrim.set_visible(true);
        self.float_host.set_visible(true);
        float.grab_focus();

        self.floating = Some(Floating::Book(ctrl));
    }

    /// The series float, opened from the book page's Series link.
    fn open_series_floating(
        &mut self,
        series: String,
        first_author: String,
        sender: &ComponentSender<Self>,
    ) {
        self.close_floating();

        let ctrl = SeriesFloatModel::builder()
            .launch((self.catalog.clone(), series, first_author))
            .forward(sender.input_sender(), |out| match out {
                SeriesFloatOut::Close => AppMsg::CloseBookDialog,
                SeriesFloatOut::OpenBook { book_id } => AppMsg::FloatOpenFull { book_id },
            });

        let float = ctrl.widget().clone();
        float.set_size_request(560, 560);
        float.set_hexpand(false);
        float.set_vexpand(false);
        float.set_halign(gtk::Align::Center);
        float.set_valign(gtk::Align::Center);
        self.float_host.append(&float);
        self.float_scrim.set_visible(true);
        self.float_host.set_visible(true);
        float.grab_focus();

        self.floating = Some(Floating::Series(ctrl));
    }

    fn build_page(
        catalog: &Arc<Catalog>,
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
            Route::AuthorPage { author } => {
                let ctrl = AuthorPageModel::builder()
                    .launch((catalog.clone(), author.clone()))
                    .forward(sender.input_sender(), |out| match out {
                        AuthorPageOut::OpenBook { book_id } => {
                            AppMsg::Push(Route::BookPage { book_id })
                        }
                        AuthorPageOut::OpenBookDialog { book_id } => {
                            AppMsg::OpenBookDialog { book_id }
                        }
                    });
                PageSlot::Author(ctrl)
            }
            Route::BookPage { book_id } => {
                let id = *book_id;
                let ctrl = BookPageModel::builder()
                    .launch((catalog.clone(), id))
                    .forward(sender.input_sender(), move |out| match out {
                        BookPageOut::Back => AppMsg::Back,
                        BookPageOut::OpenReader => AppMsg::OpenReader { book_id: id },
                        BookPageOut::OpenAuthor { name } => {
                            AppMsg::Push(Route::AuthorPage { author: name })
                        }
                        BookPageOut::OpenBook { book_id } => AppMsg::FloatOpenFull { book_id },
                        BookPageOut::OpenSeries {
                            series,
                            first_author,
                        } => AppMsg::OpenSeriesFloat {
                            series,
                            first_author,
                        },
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
                        ReaderOut::OpenAuthor { name } => {
                            AppMsg::Push(Route::AuthorPage { author: name })
                        }
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

        self.detach_current(content_host);

        self.route = route;
        let page = self.take_or_build(sender);
        content_host.append(&page.widget());
        self.page = Some(page);
        sync_content_classes(content_host, &self.route, self.show_back_chip());
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

            // One root overlay: main app under it, then a dimmed in-app book
            // panel, then toasts on top.
            #[name = "root_overlay"]
            gtk::Overlay {
                add_overlay = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,
                    set_halign: gtk::Align::End,
                    set_valign: gtk::Align::End,
                    add_css_class: "kalam-toast-host",
                    // Must not swallow clicks meant for the app beneath.
                    set_can_target: true,
                    // With no toasts up, this overlay child has no content and
                    // would be allocated 0x0 — an invalid rectangle as far as
                    // pixman is concerned. Hiding it until a toast exists keeps
                    // it out of the layout entirely.
                    set_visible: false,

                    #[name = "toast_host"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 8,
                        set_halign: gtk::Align::End,
                        set_valign: gtk::Align::End,
                    },
                },

                #[wrap(Some)]
                set_child = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_hexpand: true,
                    set_vexpand: true,
                    #[watch]
                    set_can_target: model.floating.is_none(),

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

                            #[name = "brand"]
                            gtk::Box {
                                add_css_class: "kalam-brand",
                                set_halign: gtk::Align::Center,
                            },

                            // Equal expanding spacers above and below the nav pin
                            // it to the middle of the rail, with the logo held at
                            // the top and Settings at the bottom.
                            gtk::Box {
                                set_vexpand: true,
                                add_css_class: "kalam-nav-spacer",
                            },

                            #[name = "top_nav"]
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 0,
                                set_valign: gtk::Align::Center,
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

                        gtk::Overlay {
                            add_overlay = &gtk::Box {
                                set_halign: gtk::Align::Start,
                                set_valign: gtk::Align::Start,
                                set_margin_top: 16,
                                set_margin_start: 16,
                                #[watch]
                                set_visible: model.show_back_chip(),

                                gtk::Button {
                                    add_css_class: "kalam-back-btn",
                                    add_css_class: "kalam-back-float",
                                    set_focus_on_click: false,
                                    set_child: Some(&crate::icons::labelled(
                                        "go-previous-symbolic",
                                        16,
                                        "Back",
                                        6,
                                    )),
                                    connect_clicked => AppMsg::Back,
                                },
                            },

                            #[wrap(Some)]
                            set_child = &gtk::ScrolledWindow {
                                set_hexpand: true,
                                set_vexpand: true,
                                // GTK4 dropped the global gtk-overlay-scrolling setting;
                                // it is per-widget now. Without this the scrollbar can be
                                // a permanent widget that takes layout space and is always
                                // painted, which no CSS can hide.
                                set_overlay_scrolling: true,
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
            Ok(c) => Arc::new(c),
            Err(err) => {
                // Raised before the overlay exists; notify queues it.
                crate::notify::error("Could not open the library", &err.to_string());
                Arc::new(Catalog::open().expect("catalog open"))
            }
        };

        let initial_route = Route::Module(NavItem::Home);
        let page = Self::build_page(&catalog, &initial_route, &sender);

        let float_scrim = gtk::Box::new(gtk::Orientation::Vertical, 0);
        float_scrim.add_css_class("kalam-float-scrim");
        float_scrim.set_halign(gtk::Align::Fill);
        float_scrim.set_valign(gtk::Align::Fill);
        float_scrim.set_hexpand(true);
        float_scrim.set_vexpand(true);
        float_scrim.set_can_target(true);
        float_scrim.set_visible(false);

        let float_host = gtk::Box::new(gtk::Orientation::Vertical, 0);
        float_host.add_css_class("kalam-float-stage");
        float_host.set_halign(gtk::Align::Center);
        float_host.set_valign(gtk::Align::Center);
        float_host.set_margin_top(24);
        float_host.set_margin_bottom(24);
        float_host.set_margin_start(24);
        float_host.set_margin_end(24);
        float_host.set_size_request(720, 420);
        float_host.set_overflow(gtk::Overflow::Visible);
        float_host.set_can_target(true);
        float_host.set_visible(false);

        let cache_token = catalog.change_token();
        let model = AppModel {
            catalog,
            route: initial_route,
            history: Vec::new(),
            sidebar_override: None,
            page: Some(page),
            floating: None,
            float_scrim: float_scrim.clone(),
            float_host: float_host.clone(),
            cache: Vec::new(),
            cache_token,
        };

        let widgets = view_output!();
        widgets.root_overlay.add_overlay(&float_scrim);
        widgets.root_overlay.add_overlay(&float_host);

        let block_float_clicks = gtk::GestureClick::new();
        block_float_clicks.connect_pressed(|_, _, _, _| {});
        float_scrim.add_controller(block_float_clicks);

        let close_float_key = gtk::EventControllerKey::new();
        let s_key = sender.clone();
        close_float_key.connect_key_pressed(move |_, keyval, _, _| {
            use gtk::gdk::Key;
            if float_host.is_visible()
                && (keyval == Key::q || keyval == Key::Q || keyval == Key::Escape)
            {
                s_key.input(AppMsg::CloseBookDialog);
                return gtk::glib::Propagation::Stop;
            }
            gtk::glib::Propagation::Proceed
        });
        root.add_controller(close_float_key);

        // From here on, any notify::* call lands on screen.
        crate::notify::attach(widgets.toast_host.clone());

        // Extracted-book caches are rebuilt on demand, so anything orphaned or
        // untouched for a fortnight is pure waste on a small disk.
        {
            // `catalog` was moved into the model above; use the model's handle.
            let uuids = model.catalog.all_uuids().unwrap_or_default();
            let freed = crate::paths::prune_reader_cache(&uuids, 14);
            if freed > 1024 * 1024 {
                crate::notify::info(
                    "Cleaned up reader cache",
                    &format!("Freed {}", crate::epub_write::human_size(freed)),
                );
            }
        }

        widgets.brand.append(&brand_logo());

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
        sync_content_classes(&widgets.content_host, &model.route, model.show_back_chip());

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AppMsg::Navigate(item) => {
                let route = match item {
                    NavItem::Shelves => Route::ShelvesGrid,
                    other => Route::Module(other),
                };
                self.close_floating();
                self.swap_page(&widgets.content_host, route, false, &sender);
            }
            AppMsg::Push(route) => {
                self.close_floating();
                self.swap_page(&widgets.content_host, route, true, &sender);
            }
            AppMsg::Back => {
                if let Some(prev) = self.history.pop() {
                    self.sidebar_override = match &prev {
                        Route::BookPage { .. } => self.sidebar_override,
                        _ => None,
                    };

                    self.detach_current(&widgets.content_host);

                    self.route = prev;
                    let page = self.take_or_build(&sender);
                    widgets.content_host.append(&page.widget());
                    self.page = Some(page);
                    sync_content_classes(&widgets.content_host, &self.route, self.show_back_chip());
                }
            }
            AppMsg::OpenBookDialog { book_id } => {
                self.open_floating(book_id, &sender);
            }
            AppMsg::OpenSeriesFloat {
                series,
                first_author,
            } => {
                self.open_series_floating(series, first_author, &sender);
            }
            AppMsg::FloatOpenFull { book_id } => {
                self.close_floating();
                self.swap_page(
                    &widgets.content_host,
                    Route::BookPage { book_id },
                    true,
                    &sender,
                );
            }
            AppMsg::CloseBookDialog => {
                self.close_floating();
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
                self.close_floating();
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

/// The sidebar wordmark: the Kalam logo.
///
/// Embedded with `include_bytes!` rather than read from disk so the binary
/// stays self-contained — there is no install step that would place an asset
/// directory next to it.
fn brand_logo() -> gtk::Image {
    const LOGO: &[u8] = include_bytes!("../assets/logo.png");
    // Matches the nav glyphs, a shade larger so the mark still leads the rail.
    const LOGO_PX: i32 = 24;

    let bytes = gtk::glib::Bytes::from_static(LOGO);
    let image = match gtk::gdk::Texture::from_bytes(&bytes) {
        Ok(texture) => gtk::Image::from_paintable(Some(&texture)),
        // A corrupt asset should not stop the app from starting.
        Err(_) => gtk::Image::new(),
    };
    // gtk::Image, not gtk::Picture. A Picture's natural size is the texture's
    // own size, and both set_size_request and CSS min-width are *floors*, so a
    // 128px texture drew at 128px and stretched the whole rail. Image with
    // set_pixel_size is the one widget that treats the number as exact.
    image.set_pixel_size(LOGO_PX);
    image.set_halign(gtk::Align::Center);
    image.set_valign(gtk::Align::Center);
    image.add_css_class("kalam-brand-logo");
    image
}

fn make_nav_button(item: NavItem, active: bool) -> gtk::Button {
    // Icon only. The rail is too narrow for a readable caption, and the
    // tooltip already carries the page name.
    let icon = crate::icons::symbolic_with_classes(item.icon(), 18, &["kalam-nav-icon"]);
    icon.set_halign(gtk::Align::Center);
    icon.set_valign(gtk::Align::Center);

    let btn = gtk::Button::new();
    btn.set_child(Some(&icon));
    btn.add_css_class("kalam-nav-btn");
    btn.set_halign(gtk::Align::Center);
    btn.set_hexpand(false);
    if active {
        btn.add_css_class("active");
    }
    btn.set_tooltip_text(Some(item.label()));
    btn.set_focus_on_click(false);
    btn
}

fn sync_content_classes(content_host: &gtk::Box, route: &Route, show_back_chip: bool) {
    if route.is_reader() || matches!(route, Route::Module(NavItem::Settings)) {
        content_host.add_css_class("kalam-content-flush");
    } else {
        content_host.remove_css_class("kalam-content-flush");
    }

    if route.is_reader() {
        content_host.add_css_class("kalam-content-reader");
    } else {
        content_host.remove_css_class("kalam-content-reader");
    }

    if show_back_chip {
        content_host.add_css_class("kalam-content-with-back");
    } else {
        content_host.remove_css_class("kalam-content-with-back");
    }
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
