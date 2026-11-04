//! A game's help, read the way Freight Fate reads its guide: a menu of
//! topics, and each topic in short pages, so a screen reader never gets a
//! whole topic as one long line.
//!
//! The topics come from the help document's `## ` headings, as before.
//! Inside a topic each paragraph, and each list item, is a page. In a
//! topic, Up and Down read a page at a time, Left and Right go to the
//! topic before or after, Enter reads the rest of the topic, and Escape
//! goes back to the topic menu, on the topic the player was reading.

use std::cell::Cell;
use std::rc::Rc;

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx, InputEvent, Key, State, UiSound};

use crate::{Parlor, ParlorAction};

/// One `## ` topic, split into pages.
#[derive(Clone, Debug, PartialEq)]
pub struct Topic {
    pub title: String,
    pub pages: Vec<String>,
}

/// Split a help document into topics on its `## ` headings, and each topic
/// into pages: a paragraph (lines up to a blank line) or a list item is a
/// page. Anything before the first heading (the document's own title) is
/// not read. A smaller heading inside a topic opens the page after it, and
/// a line that ends with a colon keeps the page it introduces with it.
pub fn topics(doc: &str) -> Vec<Topic> {
    let mut out: Vec<Topic> = Vec::new();
    // The page being gathered, and whether it is only a lead-in so far
    // (a sub-heading, or a line ending in a colon).
    let mut page = String::new();
    let mut lead_in = false;
    fn flush(out: &mut [Topic], page: &mut String, lead_in: &mut bool) {
        if *lead_in {
            return;
        }
        if let Some(topic) = out.last_mut() {
            if !page.is_empty() {
                topic.pages.extend(split_long(&std::mem::take(page)));
            }
        }
        page.clear();
    }
    for raw in doc.lines() {
        if let Some(title) = raw.strip_prefix("## ") {
            lead_in = false;
            flush(&mut out, &mut page, &mut lead_in);
            out.push(Topic {
                title: title.trim().to_string(),
                pages: Vec::new(),
            });
            continue;
        }
        if out.is_empty() {
            continue;
        }
        let line = raw.trim();
        if line.is_empty() {
            flush(&mut out, &mut page, &mut lead_in);
            continue;
        }
        if line.starts_with('#') {
            // A sub-heading: its words lead into the next page.
            flush(&mut out, &mut page, &mut lead_in);
            let words = line.trim_start_matches('#').trim();
            page.push_str(words);
            if !words.ends_with(['.', ':', '?', '!']) {
                page.push('.');
            }
            lead_in = true;
            continue;
        }
        let item = list_item(line);
        if let Some(text) = item {
            flush(&mut out, &mut page, &mut lead_in);
            push_words(&mut page, text);
        } else {
            push_words(&mut page, line);
        }
        lead_in = page.ends_with(':');
    }
    lead_in = false;
    flush(&mut out, &mut page, &mut lead_in);
    out
}

/// A page longer than this is cut between sentences, so no page runs on
/// much past two or three sentences. Freight Fate's guide lines are about
/// this long.
const LONG_PAGE: usize = 320;
/// The length the cut pieces aim for.
const PIECE: usize = 260;

/// Text as pages: as it is when short, else cut between sentences into
/// pieces of about two or three sentences. For anything longer than a
/// line that is read with [`crate::Pages`].
pub fn pages_of(text: &str) -> Vec<String> {
    split_long(text.trim())
}

/// `page` as it is when short, else cut between sentences into pieces of
/// about [`PIECE`] characters. A single long sentence stays whole.
fn split_long(page: &str) -> Vec<String> {
    if page.chars().count() <= LONG_PAGE {
        return vec![page.to_string()];
    }
    let mut pieces: Vec<String> = Vec::new();
    let mut piece = String::new();
    for sentence in sentences(page) {
        let fits = piece.chars().count() + 1 + sentence.chars().count() <= PIECE;
        if !piece.is_empty() && !fits {
            pieces.push(std::mem::take(&mut piece));
        }
        push_words(&mut piece, sentence);
    }
    if !piece.is_empty() {
        pieces.push(piece);
    }
    // A short tail reads better with the piece before it.
    if pieces.len() > 1 && pieces.last().is_some_and(|p| p.chars().count() < 60) {
        let tail = pieces.pop().unwrap_or_default();
        if let Some(prev) = pieces.last_mut() {
            push_words(prev, &tail);
        }
    }
    pieces
}

/// `text` cut after each sentence: a `.`, `?` or `!` (and any closing
/// quote or bracket) followed by a space and a capital letter, digit or
/// opening quote.
fn sentences(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        if matches!(bytes[i], b'.' | b'?' | b'!') {
            let mut end = i + 1;
            while end < bytes.len() && matches!(bytes[end], b'"' | b')' | b'\'') {
                end += 1;
            }
            let next = bytes.get(end + 1).copied();
            if bytes.get(end) == Some(&b' ')
                && next.is_some_and(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'"')
            {
                out.push(text[start..end].trim());
                start = end + 1;
                i = start;
                continue;
            }
        }
        i += 1;
    }
    let rest = text[start..].trim();
    if !rest.is_empty() {
        out.push(rest);
    }
    out
}

fn push_words(page: &mut String, text: &str) {
    if !page.is_empty() {
        page.push(' ');
    }
    page.push_str(text);
}

/// The text of a list item line (`- `, `* `, `1. `), or `None`.
fn list_item(line: &str) -> Option<&str> {
    if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
        return Some(rest.trim_start());
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 {
        if let Some(rest) = line[digits..].strip_prefix(". ") {
            return Some(rest.trim_start());
        }
    }
    None
}

/// The help menu: one row per topic, Enter reads it in pages.
pub struct Help {
    menu: MenuCore<Self, Parlor>,
    topics: Rc<Vec<Topic>>,
    /// The topic the reader was last on, so Escape lands back on it.
    reading: Rc<Cell<Option<usize>>>,
}

impl Help {
    pub fn new(doc: &str) -> Help {
        Help {
            menu: MenuCore::new("Help").with_intro_help(
                "Up and Down move between help topics, Enter reads one, Escape goes back.",
            ),
            topics: Rc::new(topics(doc)),
            reading: Rc::new(Cell::new(None)),
        }
    }
}

impl Menu for Help {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "help"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        (0..self.topics.len())
            .map(|i| {
                MenuItem::new(
                    self.topics[i].title.clone(),
                    move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        let reader = Reader::new(s.topics.clone(), i, s.reading.clone());
                        ctx.push(reader);
                    },
                )
                .help("Enter reads this topic a page at a time.")
            })
            .collect()
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if let Some(i) = self.reading.take() {
            self.menu.index = i.min(self.topics.len().saturating_sub(1));
        }
        self.refresh(ctx, true);
        self.announce(ctx);
    }
}

/// One topic, a page at a time.
pub struct Reader {
    topics: Rc<Vec<Topic>>,
    topic: usize,
    page: usize,
    reading: Rc<Cell<Option<usize>>>,
}

impl Reader {
    fn new(topics: Rc<Vec<Topic>>, topic: usize, reading: Rc<Cell<Option<usize>>>) -> Reader {
        Reader {
            topics,
            topic,
            page: 0,
            reading,
        }
    }

    fn current(&self) -> &Topic {
        &self.topics[self.topic]
    }

    fn page_text(&self) -> String {
        self.current()
            .pages
            .get(self.page)
            .cloned()
            .unwrap_or_default()
    }

    /// The topic's name, how many pages it has, and its first page queued
    /// behind, as a separate line the screen reader keeps on its own.
    fn open_topic(&mut self, ctx: &mut Ctx<Parlor>, numbered: bool) {
        self.page = 0;
        let topic = self.current();
        let count = match topic.pages.len() {
            0 | 1 => String::new(),
            n => format!(" {n} pages."),
        };
        let heading = if numbered {
            format!(
                "{}, {} of {}.{count}",
                topic.title,
                self.topic + 1,
                self.topics.len()
            )
        } else {
            format!("{}.{count}", topic.title)
        };
        ctx.say(heading);
        let first = self.page_text();
        if !first.is_empty() {
            ctx.say_queued(first);
        }
    }

    fn speak_page(&mut self, ctx: &mut Ctx<Parlor>) {
        let cue = crate::menu_sounds::page(ctx);
        ctx.audio.play(cue, Bus::UI);
        let text = self.page_text();
        ctx.say(text);
    }

    fn turn_topic(&mut self, ctx: &mut Ctx<Parlor>, delta: isize) {
        let n = self.topics.len() as isize;
        self.topic = (self.topic as isize + delta).rem_euclid(n) as usize;
        self.reading.set(Some(self.topic));
        ctx.play_ui(UiSound::Move);
        self.open_topic(ctx, true);
    }
}

impl State<Parlor> for Reader {
    fn name(&self) -> &'static str {
        "help topic"
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        self.reading.set(Some(self.topic));
        self.open_topic(ctx, false);
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        let action = ctx.action_for(event);
        if action == Some(ParlorAction::Repeat) && !event.is_repeat() {
            self.speak_page(ctx);
            return;
        }
        let Some(key) = event.pressed() else {
            return;
        };
        let last = self.current().pages.len().saturating_sub(1);
        match key {
            Key::Down => {
                self.page = (self.page + 1).min(last);
                self.speak_page(ctx);
            }
            Key::Up => {
                self.page = self.page.saturating_sub(1);
                self.speak_page(ctx);
            }
            Key::Home => {
                self.page = 0;
                self.speak_page(ctx);
            }
            Key::End => {
                self.page = last;
                self.speak_page(ctx);
            }
            Key::Right | Key::PageDown => self.turn_topic(ctx, 1),
            Key::Left | Key::PageUp => self.turn_topic(ctx, -1),
            Key::Return | Key::KpEnter | Key::Space => {
                // The rest of the topic from this page, each page its own
                // line, so the screen reader can stop or review any of them.
                let cue = crate::menu_sounds::page(ctx);
                ctx.audio.play(cue, Bus::UI);
                let pages: Vec<String> = self.current().pages[self.page..].to_vec();
                for (i, text) in pages.into_iter().enumerate() {
                    if i == 0 {
                        ctx.say(text);
                    } else {
                        ctx.say_queued(text);
                    }
                }
                self.page = last;
            }
            Key::Escape => {
                ctx.play_ui(UiSound::Back);
                ctx.pop();
            }
            _ => {}
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let topic = self.current();
        let mut out = vec![
            format!(
                "Help - {} ({}/{})",
                topic.title,
                self.topic + 1,
                self.topics.len()
            ),
            String::new(),
        ];
        for (i, text) in topic.pages.iter().enumerate() {
            let marker = if i == self.page { "> " } else { "  " };
            out.push(format!("{marker}{text}"));
        }
        out
    }

    fn help(&self, _ctx: &Ctx<Parlor>) -> String {
        format!(
            "Page {} of {}. Up and Down read a page at a time. Left and Right change topic. Enter reads the rest. Escape goes back.",
            self.page + 1,
            self.current().pages.len().max(1)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menus::testing::Base;
    use hearken::Headless;

    /// What was said since the last call, one entry per line.
    fn heard(h: &mut Headless<Parlor>) -> Vec<String> {
        let said = h.spoken();
        h.clear();
        said
    }

    const DOC: &str = "# Game\n\n## Moving\n\nThe arrows move you.\nHold one to keep going.\n\nWalls thud.\n\n## Sounds\n\nThree kinds:\n- A whistle is an egg.\n- A buzz is a bee.\n\n### Golden eggs\nThey sparkle.\n\n## Last\n\nOnly page.\n";

    #[test]
    fn paragraphs_and_list_items_are_pages() {
        let t = topics(DOC);
        assert_eq!(t.len(), 3);
        assert_eq!(t[0].title, "Moving");
        assert_eq!(
            t[0].pages,
            vec![
                "The arrows move you. Hold one to keep going.",
                "Walls thud."
            ]
        );
        assert_eq!(
            t[1].pages,
            vec![
                "Three kinds: A whistle is an egg.",
                "A buzz is a bee.",
                "Golden eggs. They sparkle."
            ]
        );
        assert_eq!(t[2].pages, vec!["Only page."]);
    }

    #[test]
    fn a_long_paragraph_is_cut_between_sentences() {
        let sentence = "The cat in the pantry hunts you by the shortest way it can find.";
        let long = [sentence; 8].join(" ");
        let pieces = split_long(&long);
        assert!(pieces.len() >= 2, "{pieces:?}");
        assert_eq!(pieces.join(" "), long);
        for p in &pieces {
            assert!(p.len() <= PIECE + 70, "{p}");
            assert!(p.ends_with('.'), "{p}");
        }
        let quoted = "Walls say \"Open: up.\" Then you move. 1.5 seconds pass.";
        assert_eq!(
            sentences(quoted),
            vec![
                "Walls say \"Open: up.\"",
                "Then you move.",
                "1.5 seconds pass."
            ]
        );
    }

    #[test]
    fn numbered_items_are_pages_and_text_before_a_heading_is_dropped() {
        let t = topics("Intro\n## A\n1. One.\n2. Two.\n");
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].pages, vec!["One.", "Two."]);
    }

    fn open() -> Headless<Parlor> {
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.push(Help::new(DOC));
        h.frame(0.0);
        h.clear();
        h
    }

    #[test]
    fn a_topic_opens_with_its_name_and_first_page_then_reads_a_page_at_a_time() {
        let mut h = open();
        h.press(Key::Return);
        let said = heard(&mut h).join(" | ");
        assert_eq!(
            said,
            "Moving. 2 pages. | The arrows move you. Hold one to keep going."
        );
        h.press(Key::Down);
        assert_eq!(heard(&mut h).join(" | "), "Walls thud.");
        // The last page again at the end, never past it.
        h.press(Key::Down);
        assert_eq!(heard(&mut h).join(" | "), "Walls thud.");
        h.press(Key::Up);
        assert_eq!(
            heard(&mut h).join(" | "),
            "The arrows move you. Hold one to keep going."
        );
    }

    #[test]
    fn left_and_right_change_topic_and_escape_lands_on_it() {
        let mut h = open();
        h.press(Key::Return);
        heard(&mut h);
        h.press(Key::Right);
        let said = heard(&mut h).join(" | ");
        assert_eq!(
            said,
            "Sounds, 2 of 3. 3 pages. | Three kinds: A whistle is an egg."
        );
        h.press(Key::Right);
        assert_eq!(heard(&mut h).join(" | "), "Last, 3 of 3. | Only page.");
        h.press(Key::Right);
        assert!(heard(&mut h)[0].starts_with("Moving, 1 of 3."));
        h.press(Key::Left);
        heard(&mut h);
        h.press(Key::Escape);
        assert_eq!(h.app.top_name(), "help");
        let said = heard(&mut h).join(" | ");
        assert!(said.contains("Last"), "{said}");
    }

    #[test]
    fn enter_reads_the_rest_as_separate_lines() {
        let mut h = open();
        h.press(Key::Down);
        h.press(Key::Return);
        heard(&mut h);
        h.press(Key::Return);
        assert_eq!(
            heard(&mut h),
            vec![
                "Three kinds: A whistle is an egg.".to_string(),
                "A buzz is a bee.".to_string(),
                "Golden eggs. They sparkle.".to_string()
            ]
        );
    }
}
