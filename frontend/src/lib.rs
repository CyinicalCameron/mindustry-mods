//! Frontend application of for a Mindustry-Mods listing.
#![warn(missing_docs)]

/// Some important constant path stuff.
mod path {
    pub const ROOT: &str = "";
}

/// Simple DateTime utilities.
mod date {
    use humantime::{parse_rfc3339_weak, TimestampError};
    use js_sys::Date;
    use std::time::{Duration, SystemTime, SystemTimeError, UNIX_EPOCH};
    use thiserror::Error as ThisError;

    /// DateTime related error.
    #[derive(Debug, ThisError)]
    pub enum Error {
        /// Error which occurs if times is negative.
        #[error("computation error: {0}")]
        Computation(#[from] SystemTimeError),

        /// Error which occurs if decoding/encoding error occurs.
        #[error("formatting error: {0}")]
        Formatting(#[from] TimestampError),
    }

    fn from_tt(x: f64) -> SystemTime {
        let secs = (x as u64) / 1_000;
        let nanos = ((x as u32) % 1_000) * 1_000_000;
        UNIX_EPOCH + Duration::new(secs, nanos)
    }

    fn now() -> SystemTime {
        let x = Date::now();
        from_tt(x)
    }

    /// Parses weak rfc3339 time stamps and returns the duration since now.
    pub fn ago(date: &str) -> Result<Duration, Error> {
        let sys = parse_rfc3339_weak(date)?;
        let ago = now().duration_since(sys)?;
        Ok(ago)
    }
}

/// Mod listing functions.
mod listing {
    use super::{app::Msg, app::Page, date, markup};
    use mcore::Mod;
    use seed::{prelude::*, *};
    use serde::Deserialize;
    use std::{convert::TryFrom, iter};

    /// Makes the tiny contents/assets overview lists.
    fn tiny_list(v: &[String]) -> Node<Msg> {
        let it = v
            .iter()
            .filter_map(|x| match x.as_str() {
                "content" => None,
                string => Some(string),
            })
            .map(|x| li![attrs! { At::Class => x}, x]);

        if it.clone().take(1).next().is_some() {
            ul![it]
        } else {
            div![]
        }
    }

    /// Wraps mod meta data.
    #[derive(Deserialize, Debug, Clone)]
    pub struct ListingItem(pub Mod);

    impl ListingItem {
        /// Returns whether the mod should be rendered, given a query.
        pub fn filtering(&self, query: &str) -> bool {
            if query == "" {
                true
            } else {
                query.split_whitespace().all(|q| {
                    [
                        &self.0.author,
                        &self.0.desc,
                        &self.0.repo,
                        &self.0.readme,
                        &self.0.contents.join(" "),
                        &self.0.assets.join(" "),
                    ]
                    .iter()
                    .any(|s| s.as_str().to_lowercase().contains(q))
                })
            }
        }

        fn assets_list(&self) -> Node<Msg> {
            tiny_list(&self.0.assets)
        }

        fn contents_list(&self) -> Node<Msg> {
            tiny_list(&self.0.contents)
        }

        /// Link to the mod's archive.
        fn archive_link(&self) -> Node<Msg> {
            a![
                attrs! {
                    At::Href => self.0.archive_link(),
                    At::Target => "_self"
                },
                "zip"
            ]
        }

        /// Endpoint link as a string.
        fn _endpoint_href(&self) -> String {
            let path = self.0.repo.replace("/", "--");
            format!("m/{}.html", path).into()
        }

        /// Endpoint url query string for mod. Essentially used as an ID internally.
        pub fn endpoint_query(&self) -> String {
            let path = self.0.repo.replace("/", "--");
            format!("{}", path)
        }

        // /// Endpoint link to the locally rendered README.md
        // fn endpoint_link(&self) -> Node<Msg> {
        //     a![attrs! { At::Href => self.endpoint_href() }, self.0.name]
        // }

        /// Link to the mods repository.
        fn repo_link(&self) -> Node<Msg> {
            a![attrs! { At::Href => self.0.link }, "repository"]
        }

        /// Optional link to a wiki.
        fn wiki_link(&self) -> Node<Msg> {
            match &self.0.wiki {
                Some(link) => a![attrs! { At::Href => link }, "wiki"],
                None => a![style! { "display" => "none" }],
            }
        }

        /// The rendered `time age` string.
        fn last_commit(&self) -> Node<Msg> {
            // NOTE: may want to consider using chrono instead.
            use itertools::Itertools;
            let fmt_ago = match date::ago(&self.0.date) {
                Ok(d) => format!("{}", humantime::format_duration(d))
                    .split(" ")
                    .interleave(iter::repeat(" ago"))
                    .take(2)
                    .collect(),

                Err(date::Error::Computation(_)) => "computation error".to_string(),
                Err(date::Error::Formatting(_)) => "formatting error".to_string(),
            };

            div![attrs! { At::Class => "last-commit" }, fmt_ago]
        }

        /// Returns unicode stars.
        fn stars_el(&self) -> Node<Msg> {
            let star_count: Node<Msg> = div![
                attrs! { At::Class => "star-count" },
                format!("{}", self.0.stars)
            ];
            match usize::try_from(self.0.stars) {
                Err(_) => div![star_count, div!["err"]],
                Ok(0) => div![
                    div![attrs! { At::Class => "stars-wrapper"}, "☆"],
                    star_count,
                ],
                Ok(n) => div![
                    div![
                        attrs! { At::Class => "stars-wrapper" },
                        iter::repeat("★")
                            .take(n)
                            .map(|x| div![attrs! { At::Class => "star" }, x])
                    ],
                    star_count,
                ],
            }
        }

        /// Returns an icon link node. This is a three stage process.
        ///
        /// 1. if official icon exist use it, and this can either be
        ///   a) icon.png (automatic),
        ///   b) yaml path (manual override);
        /// 2. (most-likely) else try out the github user icon;
        /// 3. otherwise, if all fails just pick a `nothing.png` placeholder;
        fn icon(&self) -> Node<Msg> {
            match self.0.icon.as_deref() {
                Some("") | None => {
                    let base = "https://github.com".to_string();
                    let icon = match self.0.repo.split("/").next() {
                        Some(user) => base + "/" + user + ".png?size=64",
                        None => "images/nothing.png".into(),
                    };
                    button![
                        simple_ev(Ev::Click, Msg::Route(Page::Overview(self.endpoint_query()))),
                        img![attrs! {
                            At::Src => &icon,
                            // At::Custom("loading".into()) => "lazy",
                        },]
                    ]
                }

                Some(p) => {
                    let i = format!(
                        "{}/{}/master/{}",
                        "https://raw.githubusercontent.com", self.0.repo, p
                    );
                    button![
                        simple_ev(Ev::Click, Msg::Route(Page::Overview(self.endpoint_query()))),
                        img![attrs! {
                            At::Src => i,
                            At::OnError => "this.src='images/nothing.png'",
                            // At::Custom("loading".into()) => "lazy",
                        }]
                    ]
                }
            }
        }

        /// Description paragraph of the mode for the listing.
        fn description(&self) -> Node<Msg> {
            p![
                style! { St::Background => "#0f0f0f" },
                attrs! { At::Class => "description" },
                match self.0.desc_markup.as_ref() {
                    Some(x) => markup::from_str(x),
                    None => vec![],
                }
            ]
        }

        /// The rendered author.
        fn by_author(&self) -> Node<Msg> {
            div![
                attrs! { At::Class => "by-author" },
                style! { St::Opacity => "60%" },
                "by ",
                markup::from_str(self.0.author_markup.as_deref().unwrap_or("null"))
            ]
        }

        /// The rendered version number.
        fn v_number(&self) -> Node<Msg> {
            let pre = if self.0.version.is_some() { "v" } else { "" };
            let num = self.0.version.as_ref().map(|x| x.as_str()).unwrap_or("");
            div![
                attrs! { At::Class => "v-number" },
                style! {
                    St::Color => "#484848",
                    St::Opacity => "50%",
                },
                pre,
                markup::from_str(num)
            ]
        }

        /// The thing the user will probably click on.
        fn title_link(&self) -> Node<Msg> {
            let name = &self.0.name_markup;
            div![
                attrs! { At::Class => "title-link" },
                button![
                    style! { St::Background => "#282828" },
                    simple_ev(Ev::Click, Msg::Route(Page::Overview(self.endpoint_query()))),
                    markup::from_str(name),
                ]
            ]
        }

        /// Title (name) of the mod in the listing.
        fn listing_title(&self) -> Node<Msg> {
            div![
                attrs! { At::Class => "title-box" },
                self.title_link(),
                self.by_author(),
                self.v_number(),
                self.last_commit()
            ]
        }

        /// Returns the `Node<Msg>` for the listing.
        pub fn listing_item(&self) -> Node<Msg> {
            div![
                attrs! { At::Class => "outside" },
                div![
                    attrs! { At::Class => "wrapper" },
                    div![attrs! { At::Class => "box icon" }, self.icon()],
                    div![attrs! { At::Class => "box name" }, self.listing_title()],
                    div![attrs! { At::Class => "box desc" }, self.description()],
                    div![
                        attrs! { At::Class => "box links" },
                        self.repo_link(),
                        self.archive_link(),
                        self.wiki_link(),
                    ],
                    div![attrs! { At::Class => "box assets" }, self.assets_list()],
                    div![attrs! { At::Class => "box contents" }, self.contents_list()],
                    div![attrs! { At::Class => "box stars" }, self.stars_el()],
                ]
            ]
        }

        /// Returns the `Node<Msg>` for the overview/readme page.
        pub fn overview_item(&self) -> Node<Msg> {
            div! {
                div![
                    class!["outside"],
                    button![
                        style! { St::Background => "#282828" },
                        simple_ev(Ev::Click, Msg::Route(Page::Listing)),
                        "back",
                    ],
                ],

                self.listing_item(),

                div![
                    class!["outside"],
                    div! [
                        class!("markdown"),
                        md!(&self.0.readme)
                    ]
                ]
            }
        }
    }
}

/// Color markup rendering layer.
mod markup {
    use super::app::Msg;
    use mcore::{
        color::{Color, Name},
        markup::Markup,
    };
    use seed::{prelude::*, Style, *};

    trait ToStyle {
        fn to_style(&self) -> Style;
    }

    impl ToStyle for Color {
        fn to_style(&self) -> Style {
            style! { St::Color => self.to_string() }
        }
    }

    /// Converts input markup string to html nodes.
    pub fn from_str(input: &str) -> Vec<Node<Msg>> {
        let mut colors: Vec<Color> = vec![];
        let last = |v: &[Color]| {
            v.last()
                .cloned()
                .unwrap_or(Color::from(Name::White))
                .to_style()
        };
        let mut output: Vec<Node<Msg>> = vec![];
        for x in Markup::from_str(input).unwrap_or(("", vec![])).1 {
            use Markup::*;
            match x {
                HexColor { r, g, b, a } => match a {
                    Some(a) => colors.push([r, g, b, a].into()),
                    None => colors.push([r, g, b].into()),
                },
                Named(input) => colors.push(input.into()),
                Popped => {
                    colors.pop();
                }
                Text(text) => output.push(span![last(&colors), text]),
                Escaped => output.push(span![last(&colors), "["]),
                NewLine => output.push(span![last(&colors), "\n"]),
            }
        }
        output
    }
}

/// Base model/msg for application.
pub mod app {
    use super::{listing::ListingItem, path::ROOT};
    use mcore::MOD_VERSION;
    use seed::{prelude::*, *};

    /// Package version string.
    const VERSION: &str = env!("CARGO_PKG_VERSION");

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = console)]
        fn log(s: &str);
    }

    struct MaxCount(usize);

    impl Default for MaxCount {
        /// Default is 8 for no specific reason.
        fn default() -> Self {
            Self(8)
        }
    }

    /// Represents a separete page within the app.
    #[derive(Clone, Debug, PartialEq)]
    pub enum Page {
        /// Overview of specific mod.
        ///
        /// This is stored as a string,
        /// because it could come from the url query or from the
        /// loaded mod itself when clicked onto.
        ///
        /// It's simply string of `"user--repo"` which should prevent
        /// all imaginable collisions, those strings come from the repository
        /// strings, because they're already URL encoded.
        Overview(String),

        /// Listing of mod items.
        Listing,
    }

    impl Default for Page {
        fn default() -> Self {
            Self::Listing
        }
    }

    #[derive(Default)]
    struct Model {
        /// A vector of mod data.
        data: Vec<ListingItem>,

        /// Button sort state of listing.
        sorting: Sorting,

        /// Filtering characters entered by user.
        filtering: Option<String>,

        /// Active page which should be rendered.
        page: Page,

        /// Maximum number of elements to render in listing; this is
        /// done mainly to decrease load time, which also includes
        /// time required to sort the listing and time required
        /// to filter the listing.
        ///
        /// This is a performance optimization, required by sending
        /// `Node<Msg>` through seed-rs being a very expensive operation,
        /// for example a list of 100 mods could easily take 300ms to render.
        ///
        /// This also cuts down the number of icon which needs to
        /// be loaded at once.
        max_count: MaxCount,
    }

    impl Model {
        /// Returns listing of mods, sorted by the sort state.
        fn listing(&self) -> Vec<Node<Msg>> {
            let mut data = self.data.clone();
            match self.sorting {
                Sorting::Commit => data.sort_by_key(|x| x.0.date_tt as u32),
                Sorting::Stars => data.sort_by_key(|x| x.0.stars),
            }
            data.reverse();
            data.iter()
                .filter(|x| {
                    self.filtering
                        .as_ref()
                        .map_or(true, |f| x.filtering(f.as_str()))
                })
                .take(self.max_count.0)
                .map(|x| x.listing_item())
                .collect()
        }
    }

    /// Sorting of listing.
    #[derive(Debug, Clone, PartialEq)]
    pub enum Sorting {
        /// Github stars.
        Stars,

        /// Commit datetime.
        Commit,
    }

    impl Default for Sorting {
        fn default() -> Self {
            Self::Commit
        }
    }

    /// Main message type for seed-rs application.
    #[derive(Debug, Clone)]
    pub enum Msg {
        /// On scroll triggered event.
        Scroll {
            /// Scroll position
            scroll: i64,

            /// Window height
            height: i64,

            /// Document offset
            offset: i64,
        },

        /// Fetched mod data for listing.
        FetchData(fetch::ResponseDataResult<Vec<ListingItem>>),

        /// Set sorting order of listing.
        SetSort(Sorting),

        /// Filter by (words?) in string for listing.
        FilterWords(String),

        /// Change the route and then change the page.
        Route(Page),

        /// Change page without changing the route. (used when page is
        /// already the URL and pushing a new route would be incorrect)
        ChangePage(Page),

        /// Scroll event failed, reason untracked, so just disable scroll
        /// related behavior.
        ScrollError,
    }

    fn scroll_to_top() {
        scroll_to_y(0);
    }

    fn scroll_to_y(y: i64) {
        web_sys::window()
            .unwrap()
            .scroll_to_with_x_and_y(0.0, y as _);
    }

    fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
        match msg {
            Msg::Route(Page::Overview(name)) => {
                let q = format!("mod={}", name);
                let url = seed::Url::new(vec![ROOT]).search(&q);
                seed::push_route(url);
                scroll_to_top();
                orders
                    .skip()
                    .send_msg(Msg::ChangePage(Page::Overview(name)));
            }

            Msg::Route(Page::Listing) => {
                let url = seed::Url::new(vec![ROOT]);
                seed::push_route(url);
                model.max_count = Default::default();
                orders.skip().send_msg(Msg::ChangePage(Page::Listing));
            }

            Msg::ChangePage(page) => {
                model.page = page;
            }

            Msg::Scroll {
                scroll,
                height,
                offset,
            } => {
                if (height + scroll) > (offset - 50) {
                    model.max_count.0 += model.max_count.0;
                }
            }

            Msg::FetchData(data) => match data {
                Ok(x) => model.data = x,
                Err(e) => {
                    log("modmeta loading failed");
                    log(&format!("{:?}", e));
                }
            },

            Msg::SetSort(sorting) => {
                model.max_count = Default::default();
                model.sorting = sorting
            }

            Msg::FilterWords(words) => {
                model.max_count = Default::default();
                model.filtering = Some(words);
            }

            Msg::ScrollError => {
                log("ERROR: scroll error");
                model.max_count.0 = model.data.len();
            }
        }
    }

    fn view(model: &Model) -> impl View<Msg> {
        div! {
            attrs! { At::Class => "app" },

            // header section
            header![
                match &model.page {
                    Page::Listing => h1!["Mindustry Mods"],
                    Page::Overview(_) => a![
                        // attrs! { At::Href => "/" },
                        simple_ev(Ev::Click, Msg::Route(Page::Listing)),
                        h1!["Mindustry Mods"]
                    ]
                },
                a![
                    attrs! { At::Href => "https://github.com/SimonWoodburyForget/mindustry-mods" },
                    img![attrs! {
                        At::Src => "images/GitHub-Mark/PNG/GitHub-Mark-Light-64px.png",
                    }]
                ]
            ],

            // button and search bar section
            // (or nothing if overview mode)
            match &model.page {
                Page::Listing => div! {
                    attrs! { At::Class => "inputs" },
                    input![
                        attrs! {
                            "placeholder" => "search",
                            At::Value => &model.filtering.as_deref().unwrap_or(""),
                        },
                        input_ev(Ev::Input, Msg::FilterWords)
                    ],
                    div! {
                        attrs! { At::Class => "buttons" },
                        p!["Order by : "],
                        button![
                            attrs! { At::Class => if model.sorting == Sorting::Stars {"active"} else {""}},
                            simple_ev(Ev::Click, Msg::SetSort(Sorting::Stars)),
                            "stars"
                        ],
                        button![
                            attrs! { At::Class => if model.sorting == Sorting::Commit {"active"} else {""}},
                            simple_ev(Ev::Click, Msg::SetSort(Sorting::Commit)),
                            "commit"
                        ],
                    }
                },
                Page::Overview(_) => div![],
            },

            // listing or overview section
            match &model.page {
                Page::Overview(ref value) => match &model.data.iter()
                    .find(|x| x.endpoint_query().as_str() == value.as_str()) {
                        Some(item) => item.overview_item(),
                        None => div! {
                            attrs! { At::Class => "listing-container" },
                            model.listing(),
                        }
                    }

                Page::Listing => div! {
                    attrs! { At::Class => "listing-container" },
                    model.listing(),
                }
            }
        }
    }

    async fn fetch_data() -> Result<Msg, Msg> {
        Request::new(format!("data/modmeta.{}.json", MOD_VERSION))
            .method(Method::Get)
            .fetch_json_data(Msg::FetchData)
            .await
    }

    /// Initialize data.
    fn after_mount(_: Url, orders: &mut impl Orders<Msg>) -> AfterMount<Model> {
        orders.perform_cmd(fetch_data());
        AfterMount::default()
    }

    /// Parse query and change the page to overview if there's a mod param, or
    /// just to to listing otherwise.
    fn routes(url: Url) -> Option<Msg> {
        let find_mod = |query: String| {
            query.split("&").find_map(|pairs| {
                let mut it = pairs.split("=");
                let key = it.next().filter(|&k| k == "mod");
                let value = it.next().map(|x| x.to_string());
                key.and(value)
            })
        };

        url.search
            .and_then(find_mod)
            .map(|name| Some(Msg::ChangePage(Page::Overview(name))))
            .unwrap_or(Some(Msg::ChangePage(Page::Listing)))
    }

    fn events(_model: &Model) -> Vec<EventHandler<Msg>> {
        let some_window = web_sys::window().and_then(|window| {
            let height = window.inner_height().ok()?.as_f64()?.round() as i64;
            Some((window, height))
        });

        vec![ev(Ev::Scroll, |_| {
            some_window
                .and_then(|(window, height)| {
                    let offset = window.document()?.body()?.offset_height() as i64;
                    let scroll = window.scroll_y().ok()?.round() as i64;
                    Some(Msg::Scroll {
                        scroll,
                        offset,
                        height,
                    })
                })
                .unwrap_or(Msg::ScrollError)
        })]
    }

    /// Entry point of app.
    #[wasm_bindgen(start)]
    pub fn render() {
        log(&format!("frontend v{}", VERSION));
        log(&format!("data v{} loaded", MOD_VERSION));
        seed::App::builder(update, view)
            .window_events(events)
            .routes(routes)
            .after_mount(after_mount)
            .build_and_start();
    }
}
