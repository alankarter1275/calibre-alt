//! Types and message definitions for the comics reader component.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReadingDirection {
    #[default]
    Ltr,
    Rtl,
    Webtoon,
}

impl ReadingDirection {
    pub fn label(self) -> &'static str {
        match self {
            ReadingDirection::Ltr => "LTR (Left → Right)",
            ReadingDirection::Rtl => "RTL (Right → Left / Manga)",
            ReadingDirection::Webtoon => "Webtoon (Vertical Scroll)",
        }
    }

    pub fn next(self) -> Self {
        match self {
            ReadingDirection::Ltr => ReadingDirection::Rtl,
            ReadingDirection::Rtl => ReadingDirection::Webtoon,
            ReadingDirection::Webtoon => ReadingDirection::Ltr,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FitMode {
    #[default]
    Width,
    Height,
    Original,
}

impl FitMode {
    pub fn label(self) -> &'static str {
        match self {
            FitMode::Width => "Fit Width",
            FitMode::Height => "Fit Height",
            FitMode::Original => "Original Size",
        }
    }

    pub fn next(self) -> Self {
        match self {
            FitMode::Width => FitMode::Height,
            FitMode::Height => FitMode::Original,
            FitMode::Original => FitMode::Width,
        }
    }
}

#[derive(Debug)]
pub enum ComicsReaderMsg {
    SetPage(usize),
    NextPage,
    PrevPage,
    ToggleDirection,
    ToggleFitMode,
    #[allow(dead_code)]
    ToggleChrome,
    Close,
    #[allow(dead_code)]
    PageLoaded { index: usize, data: Vec<u8> },
}

#[derive(Debug)]
pub enum ComicsReaderOut {
    Close,
}
