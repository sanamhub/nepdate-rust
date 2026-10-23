//! The calendar's three zoom levels as one state machine (WR-03 AC6, WR-10 AC2). Pure Rust, no
//! Slint and no OS calls (ADR-0015): the markup draws what this says and sends clicks and keys
//! back. The range is 1901 to 2199 at every level, and nothing here panics at either edge.

use nepdate::{Month, NepaliDate};

use crate::app::month_view::step_month;

/// The first and last BS year of the range (WR-03 AC6).
const FIRST_YEAR: u16 = 1901;
const LAST_YEAR: u16 = 2199;
/// The first year of the first and last decade views: 1900 to 1909, and 2190 to 2199.
const FIRST_DECADE: u16 = 1900;
const LAST_DECADE: u16 = 2190;
/// The year and decade views are 4 columns by 3 rows.
const COLUMNS: u8 = 4;
const CELLS: u8 = 12;

/// Which level the calendar shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zoom {
    /// One month as a day grid.
    Month,
    /// The 12 months of one year as a 4×3 grid.
    Year,
    /// 12 years as a 4×3 grid: the decade, with one year on each side.
    Decade,
}

/// An arrow key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrow {
    Left,
    Right,
    Up,
    Down,
}

/// The zoom state: the level, the viewed month, the decade shown, and the grid's roving cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoomState {
    zoom: Zoom,
    year: u16,
    month: Month,
    /// The first year of the decade the decade view shows; a multiple of 10.
    decade: u16,
    /// The cursor's day of the month (1-based) in the month view, the cell (0 to 11) in the other
    /// two. It is where Tab lands and where Enter acts.
    cursor: u8,
}

/// The decade a year belongs to, as the first year of the decade.
fn decade_of(year: u16) -> u16 {
    year / 10 * 10
}

impl ZoomState {
    /// The month view on `today`'s month, with the cursor on today.
    #[must_use]
    pub fn new(today: NepaliDate) -> Self {
        Self {
            zoom: Zoom::Month,
            year: today.year(),
            month: today.month(),
            decade: decade_of(today.year()),
            cursor: today.day(),
        }
    }

    #[must_use]
    pub fn zoom(self) -> Zoom {
        self.zoom
    }

    /// The year the month and year views show.
    #[must_use]
    pub fn year(self) -> u16 {
        self.year
    }

    /// The viewed month: the one the month view shows and the year view marks.
    #[must_use]
    pub fn month(self) -> Month {
        self.month
    }

    /// The viewed month as `(year, month)`.
    #[must_use]
    pub fn viewed(self) -> (u16, Month) {
        (self.year, self.month)
    }

    /// The first year of the decade the decade view shows.
    #[must_use]
    pub fn decade(self) -> u16 {
        self.decade
    }

    /// The cursor: a day of the month in the month view, a cell in the other views.
    #[must_use]
    pub fn cursor(self) -> u8 {
        self.cursor
    }

    /// The date under the cursor in the month view.
    #[must_use]
    pub fn cursor_date(self) -> Option<NepaliDate> {
        NepaliDate::new(self.year, self.month.number(), self.cursor).ok()
    }

    /// The 12 cells of the decade view: the year before the decade, the ten years of it, and the
    /// year after. A year outside 1901 to 2199 is `None`, so it is not offered as a choice.
    #[must_use]
    pub fn decade_years(self) -> [Option<u16>; 12] {
        let first = i32::from(self.decade) - 1;
        std::array::from_fn(|i| {
            let year = first + i32::try_from(i).unwrap_or(0);
            u16::try_from(year)
                .ok()
                .filter(|year| (FIRST_YEAR..=LAST_YEAR).contains(year))
        })
    }

    /// Clicking the title: month to year to decade. The decade view's title does nothing.
    pub fn title_clicked(&mut self) {
        match self.zoom {
            Zoom::Month => {
                self.zoom = Zoom::Year;
                self.cursor = self.month.number() - 1;
            }
            Zoom::Year => {
                self.zoom = Zoom::Decade;
                self.decade = decade_of(self.year);
                self.cursor = self.cell_of_year(self.year).unwrap_or(1);
            }
            Zoom::Decade => {}
        }
    }

    /// Picking a month in the year view opens it in the month view. `false` outside the year view.
    pub fn pick_month(&mut self, month: Month) -> bool {
        if self.zoom != Zoom::Year {
            return false;
        }
        self.month = month;
        self.zoom = Zoom::Month;
        self.cursor = 1;
        true
    }

    /// Picking a year in the decade view opens its year view. `false` outside the decade view or
    /// for a year that is not one of the shown choices.
    pub fn pick_year(&mut self, year: u16) -> bool {
        if self.zoom != Zoom::Decade || self.cell_of_year(year).is_none() {
            return false;
        }
        self.year = year;
        self.zoom = Zoom::Year;
        self.cursor = self.month.number() - 1;
        true
    }

    /// A click on cell `index` of the year or decade view: the cursor goes there and the cell is
    /// picked. `false` for the month view, an empty cell or an index past the 12 cells.
    pub fn pick_cell(&mut self, index: u8) -> bool {
        if index >= CELLS {
            return false;
        }
        let before = self.cursor;
        self.cursor = index;
        let picked = self.pick_cursor();
        if !picked {
            self.cursor = before;
        }
        picked
    }

    /// Enter on the cursor in the year or decade view. The month view's Enter opens the day
    /// detail instead, which is the caller's. `false` when nothing was picked.
    pub fn pick_cursor(&mut self) -> bool {
        match self.zoom {
            Zoom::Month => false,
            Zoom::Year => {
                Month::try_from(self.cursor + 1).is_ok_and(|month| self.pick_month(month))
            }
            Zoom::Decade => self
                .decade_years()
                .get(usize::from(self.cursor))
                .copied()
                .flatten()
                .is_some_and(|year| self.pick_year(year)),
        }
    }

    /// Whether the previous (`delta < 0`) or next (`delta > 0`) control has anywhere to go. The
    /// control that would leave the range is disabled (WR-03 AC6).
    #[must_use]
    pub fn can_step(self, delta: i32) -> bool {
        match self.zoom {
            Zoom::Month => step_month(self.year, self.month, delta).is_some(),
            Zoom::Year => step_year(self.year, delta).is_some(),
            Zoom::Decade => step_decade(self.decade, delta).is_some(),
        }
    }

    /// Previous or next: one month, one year or ten years. `false` at the edge, with no change.
    pub fn step(&mut self, delta: i32) -> bool {
        match self.zoom {
            Zoom::Month => {
                let Some((year, month)) = step_month(self.year, self.month, delta) else {
                    return false;
                };
                self.year = year;
                self.month = month;
                self.cursor = self.cursor.min(self.month_length());
            }
            Zoom::Year => {
                let Some(year) = step_year(self.year, delta) else {
                    return false;
                };
                self.year = year;
            }
            Zoom::Decade => {
                let Some(decade) = step_decade(self.decade, delta) else {
                    return false;
                };
                self.decade = decade;
                self.cursor = self.first_year_cell_from(self.cursor);
            }
        }
        true
    }

    /// Escape steps back one level: decade to year to month. `false` in the month view, where
    /// Escape closes the popup instead.
    pub fn escape(&mut self) -> bool {
        match self.zoom {
            Zoom::Month => false,
            Zoom::Year => {
                self.zoom = Zoom::Month;
                self.cursor = 1;
                true
            }
            Zoom::Decade => {
                self.zoom = Zoom::Year;
                self.cursor = self.month.number() - 1;
                true
            }
        }
    }

    /// Today, from any level: today's month in the month view, the cursor on today.
    pub fn today(&mut self, today: NepaliDate) {
        self.jump_to(today);
    }

    /// Opens `date`'s month in the month view with the cursor on it. Go to date, the upcoming
    /// holidays and a reopen on a new day all land here.
    pub fn jump_to(&mut self, date: NepaliDate) {
        self.zoom = Zoom::Month;
        self.year = date.year();
        self.month = date.month();
        self.decade = decade_of(date.year());
        self.cursor = date.day();
    }

    /// Whether the Today pill shows: everywhere except today's month in the month view.
    #[must_use]
    pub fn today_pill_shown(self, today: NepaliDate) -> bool {
        !(self.zoom == Zoom::Month && self.viewed() == (today.year(), today.month()))
    }

    /// The popup was hidden and shown again: back to the month view of the same month (WR-02 AC2),
    /// with the cursor on today when today is in it.
    pub fn reopened(&mut self, today: NepaliDate) {
        self.zoom = Zoom::Month;
        self.decade = decade_of(self.year);
        self.cursor = if self.viewed() == (today.year(), today.month()) {
            today.day()
        } else {
            1
        };
    }

    /// An arrow key moves the cursor (WR-10 AC2): a day in the month view, a month or a year in
    /// the zoom views. Past the edge of the grid it carries into the next or previous page, and
    /// at the end of the range it stays.
    pub fn move_cursor(&mut self, arrow: Arrow) {
        match self.zoom {
            Zoom::Month => self.move_day(arrow),
            Zoom::Year => self.move_month_cell(arrow),
            Zoom::Decade => self.move_year_cell(arrow),
        }
    }

    fn month_length(self) -> u8 {
        NepaliDate::new(self.year, self.month.number(), 1).map_or(30, NepaliDate::month_length)
    }

    /// The decade view cell that holds `year`, if it is one of the 12 shown choices.
    fn cell_of_year(self, year: u16) -> Option<u8> {
        self.decade_years()
            .iter()
            .position(|cell| *cell == Some(year))
            .and_then(|cell| u8::try_from(cell).ok())
    }

    /// `cell` if it holds a year, otherwise the first cell that does. A decade page at either end
    /// of the range has empty cells, and the cursor must not rest on one.
    fn first_year_cell_from(self, cell: u8) -> u8 {
        let years = self.decade_years();
        if years.get(usize::from(cell)).copied().flatten().is_some() {
            return cell;
        }
        years
            .iter()
            .position(Option::is_some)
            .and_then(|first| u8::try_from(first).ok())
            .unwrap_or(0)
    }

    fn move_day(&mut self, arrow: Arrow) {
        let days = match arrow {
            Arrow::Left => -1,
            Arrow::Right => 1,
            Arrow::Up => -7,
            Arrow::Down => 7,
        };
        let Some(date) = self.cursor_date() else {
            return;
        };
        if let Ok(target) = date.checked_add_days(days) {
            self.year = target.year();
            self.month = target.month();
            self.decade = decade_of(target.year());
            self.cursor = target.day();
        }
    }

    fn move_month_cell(&mut self, arrow: Arrow) {
        let (page, cell) = carry(self.cursor, arrow);
        let Some(year) = step_year_by(self.year, page) else {
            return;
        };
        self.year = year;
        self.decade = decade_of(year);
        self.cursor = cell;
    }

    fn move_year_cell(&mut self, arrow: Arrow) {
        let Some(year) = self
            .decade_years()
            .get(usize::from(self.cursor))
            .copied()
            .flatten()
        else {
            return;
        };
        let by = match arrow {
            Arrow::Left => -1,
            Arrow::Right => 1,
            Arrow::Up => -i32::from(COLUMNS),
            Arrow::Down => i32::from(COLUMNS),
        };
        let Some(target) = step_year_by(year, by) else {
            return;
        };
        if self.cell_of_year(target).is_none() {
            self.decade = decade_of(target);
        }
        self.cursor = self.cell_of_year(target).unwrap_or(1);
    }
}

/// An arrow in a 4×3 grid: how many pages it carries (-1, 0 or 1) and the cell it lands on. Left
/// from the first cell and Right from the last carry to the neighbouring page; Up from the top
/// row and Down from the bottom row land on the same column of it.
fn carry(cell: u8, arrow: Arrow) -> (i32, u8) {
    match arrow {
        Arrow::Left if cell == 0 => (-1, CELLS - 1),
        Arrow::Left => (0, cell - 1),
        Arrow::Right if cell == CELLS - 1 => (1, 0),
        Arrow::Right => (0, cell + 1),
        Arrow::Up if cell < COLUMNS => (-1, cell + CELLS - COLUMNS),
        Arrow::Up => (0, cell - COLUMNS),
        Arrow::Down if cell >= CELLS - COLUMNS => (1, cell + COLUMNS - CELLS),
        Arrow::Down => (0, cell + COLUMNS),
    }
}

/// `year` one step back or forward, or `None` at 1901 or 2199.
fn step_year(year: u16, delta: i32) -> Option<u16> {
    step_year_by(year, delta.signum())
}

/// `year` moved by `by` years, or `None` outside 1901 to 2199. `0` is a no-op that still checks
/// the range.
fn step_year_by(year: u16, by: i32) -> Option<u16> {
    let target = u16::try_from(i32::from(year) + by).ok()?;
    (FIRST_YEAR..=LAST_YEAR).contains(&target).then_some(target)
}

/// The decade ten years back or forward, or `None` past 1900 to 1909 or 2190 to 2199.
fn step_decade(decade: u16, delta: i32) -> Option<u16> {
    let target = u16::try_from(i32::from(decade) + delta.signum() * 10).ok()?;
    (FIRST_DECADE..=LAST_DECADE)
        .contains(&target)
        .then_some(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: u16, month: u8, day: u8) -> NepaliDate {
        NepaliDate::new(year, month, day).expect("valid BS date")
    }

    fn state_at(year: u16, month: u8, day: u8) -> ZoomState {
        ZoomState::new(date(year, month, day))
    }

    /// The year under the cursor in the decade view.
    fn year_under_cursor(state: ZoomState) -> Option<u16> {
        state.decade_years()[usize::from(state.cursor())]
    }

    #[test]
    fn a_new_state_is_the_month_view_of_today_with_the_cursor_on_today() {
        let state = state_at(2083, 7, 5);
        assert_eq!(state.zoom(), Zoom::Month);
        assert_eq!(state.viewed(), (2083, Month::Kartik));
        assert_eq!(state.decade(), 2080);
        assert_eq!(state.cursor(), 5);
        assert_eq!(state.cursor_date(), Some(date(2083, 7, 5)));
    }

    #[test]
    fn wr03_ac6_the_title_zooms_month_to_year_to_decade_and_stops() {
        let mut state = state_at(2083, 7, 5);
        state.title_clicked();
        assert_eq!(state.zoom(), Zoom::Year);
        assert_eq!(state.cursor(), 6, "the cursor starts on the viewed month");
        assert_eq!(
            state.viewed(),
            (2083, Month::Kartik),
            "zooming changes nothing viewed"
        );

        state.title_clicked();
        assert_eq!(state.zoom(), Zoom::Decade);
        assert_eq!(state.decade(), 2080);
        assert_eq!(
            year_under_cursor(state),
            Some(2083),
            "the cursor starts on the viewed year"
        );

        state.title_clicked();
        assert_eq!(state.zoom(), Zoom::Decade, "the decade title does nothing");
    }

    #[test]
    fn wr03_ac6_a_month_pick_opens_that_month_and_only_in_the_year_view() {
        let mut state = state_at(2083, 7, 5);
        assert!(!state.pick_month(Month::Baishakh), "not in the month view");
        state.title_clicked();
        assert!(state.pick_month(Month::Baishakh));
        assert_eq!(state.zoom(), Zoom::Month);
        assert_eq!(state.viewed(), (2083, Month::Baishakh));
        assert_eq!(state.cursor(), 1);

        state.title_clicked();
        state.title_clicked();
        assert!(!state.pick_month(Month::Chaitra), "not in the decade view");
        assert_eq!(state.zoom(), Zoom::Decade);
    }

    #[test]
    fn wr03_ac6_a_year_pick_opens_its_year_view_and_keeps_the_month() {
        let mut state = state_at(2083, 7, 5);
        assert!(!state.pick_year(2085), "not in the decade view");
        state.title_clicked();
        state.title_clicked();
        assert!(state.pick_year(2085));
        assert_eq!(state.zoom(), Zoom::Year);
        assert_eq!(state.year(), 2085);
        assert_eq!(state.month(), Month::Kartik);
        assert_eq!(state.cursor(), 6);
    }

    #[test]
    fn wr03_ac6_a_year_pick_rejects_a_year_that_is_not_a_choice() {
        let mut state = state_at(2083, 7, 5);
        state.title_clicked();
        state.title_clicked();
        // 2079 and 2090 are the neighbours and are choices; 2070 and 2100 are not shown.
        assert!(!state.pick_year(2070));
        assert!(!state.pick_year(2100));
        assert!(!state.pick_year(0));
        assert!(!state.pick_year(u16::MAX));
        assert_eq!(state.zoom(), Zoom::Decade);
        assert!(
            state.pick_year(2090),
            "the year after the decade is a choice"
        );
        assert_eq!(state.year(), 2090);
    }

    #[test]
    fn wr10_ac2_enter_picks_the_cursor_cell_in_the_zoom_views() {
        let mut state = state_at(2083, 7, 5);
        assert!(
            !state.pick_cursor(),
            "the month view's Enter is the day detail's"
        );

        state.title_clicked();
        state.move_cursor(Arrow::Right);
        assert!(state.pick_cursor());
        assert_eq!(
            state.viewed(),
            (2083, Month::Mangsir),
            "Kartik then Right is Mangsir"
        );
        assert_eq!(state.zoom(), Zoom::Month);

        state.title_clicked();
        state.title_clicked();
        state.move_cursor(Arrow::Right);
        assert!(state.pick_cursor());
        assert_eq!(state.zoom(), Zoom::Year);
        assert_eq!(state.year(), 2084);
    }

    #[test]
    fn wr03_ac6_a_click_on_a_cell_picks_it_and_an_empty_cell_does_nothing() {
        let mut state = state_at(2083, 7, 5);
        assert!(!state.pick_cell(0), "the month view has no cells to pick");
        state.title_clicked();
        assert!(state.pick_cell(0));
        assert_eq!(state.viewed(), (2083, Month::Baishakh));

        let mut start = state_at(1901, 1, 1);
        start.title_clicked();
        start.title_clicked();
        assert!(!start.pick_cell(0), "1899 is not shown as a choice");
        assert!(!start.pick_cell(1), "1900 is not shown as a choice");
        assert!(!start.pick_cell(12), "past the grid");
        assert_eq!(start.zoom(), Zoom::Decade);
        assert!(start.pick_cell(2), "1901");
        assert_eq!(start.year(), 1901);
    }

    #[test]
    fn wr03_ac6_previous_and_next_step_one_month_one_year_or_ten_years() {
        let mut state = state_at(2083, 7, 5);
        assert!(state.step(1));
        assert_eq!(state.viewed(), (2083, Month::Mangsir));
        assert!(state.step(-1));
        assert!(state.step(-1));
        assert_eq!(state.viewed(), (2083, Month::Ashoj));
        let mut state = state_at(2083, 12, 5);
        assert!(state.step(1));
        assert_eq!(
            state.viewed(),
            (2084, Month::Baishakh),
            "Chaitra carries into the next year"
        );

        let mut state = state_at(2083, 7, 5);
        state.title_clicked();
        assert!(state.step(1));
        assert_eq!((state.year(), state.month()), (2084, Month::Kartik));
        assert!(state.step(-1));
        assert!(state.step(-1));
        assert_eq!(state.year(), 2082);

        state.title_clicked();
        assert_eq!(state.decade(), 2080);
        assert!(state.step(1));
        assert_eq!(state.decade(), 2090);
        assert_eq!(state.year(), 2082, "the viewed year waits for a pick");
        assert!(state.step(-1));
        assert!(state.step(-1));
        assert_eq!(state.decade(), 2070);
    }

    #[test]
    fn wr03_ac6_a_month_step_keeps_the_cursor_day_inside_the_month() {
        // Day 31 of Baishakh 2081 steps back to Chaitra 2080, which has 30 days, and lands on
        // its last day.
        let mut state = state_at(2081, 1, 31);
        assert!(state.step(-1));
        assert_eq!(state.viewed(), (2080, Month::Chaitra));
        assert_eq!(state.cursor(), date(2080, 12, 1).month_length());
    }

    #[test]
    fn wr03_ac6_the_month_view_stops_at_1901_baishakh_and_2199_chaitra() {
        let mut start = state_at(1901, 1, 1);
        assert!(!start.can_step(-1));
        assert!(!start.step(-1));
        assert_eq!(start.viewed(), (1901, Month::Baishakh));
        assert!(start.can_step(1));
        assert!(start.step(1));

        let mut end = state_at(2199, 12, 30);
        assert!(!end.can_step(1));
        assert!(!end.step(1));
        assert_eq!(end.viewed(), (2199, Month::Chaitra));
        assert!(end.can_step(-1));
        assert!(end.step(-1));

        // Inside the first and last years only the edge month is blocked.
        assert!(state_at(1901, 2, 1).can_step(-1));
        assert!(state_at(2199, 11, 1).can_step(1));
    }

    #[test]
    fn wr03_ac6_the_year_view_stops_at_1901_and_2199() {
        let mut start = state_at(1901, 6, 1);
        start.title_clicked();
        assert!(!start.can_step(-1));
        assert!(!start.step(-1));
        assert_eq!(start.year(), 1901);
        assert!(start.can_step(1));

        let mut end = state_at(2199, 6, 1);
        end.title_clicked();
        assert!(!end.can_step(1));
        assert!(!end.step(1));
        assert_eq!(end.year(), 2199);
        assert!(end.can_step(-1));
    }

    #[test]
    fn wr03_ac6_the_decade_view_stops_at_the_first_and_last_decade() {
        let mut start = state_at(1901, 6, 1);
        start.title_clicked();
        start.title_clicked();
        assert_eq!(start.decade(), 1900);
        assert!(!start.can_step(-1));
        assert!(!start.step(-1));
        assert_eq!(start.decade(), 1900);
        assert!(start.can_step(1));

        let mut end = state_at(2199, 6, 1);
        end.title_clicked();
        end.title_clicked();
        assert_eq!(end.decade(), 2190);
        assert!(!end.can_step(1));
        assert!(!end.step(1));
        assert_eq!(end.decade(), 2190);
        assert!(end.can_step(-1));
    }

    #[test]
    fn wr03_ac6_years_outside_the_range_are_not_shown_as_choices() {
        let mut start = state_at(1901, 6, 1);
        start.title_clicked();
        start.title_clicked();
        let years = start.decade_years();
        assert_eq!(years[0], None, "1899");
        assert_eq!(years[1], None, "1900");
        assert_eq!(years[2], Some(1901));
        assert_eq!(years[10], Some(1909));
        assert_eq!(years[11], Some(1910), "the year after the decade");
        assert_eq!(start.cursor(), 2, "the cursor starts on 1901");

        let mut end = state_at(2199, 6, 1);
        end.title_clicked();
        end.title_clicked();
        let years = end.decade_years();
        assert_eq!(years[0], Some(2189), "the year before the decade");
        assert_eq!(years[10], Some(2199));
        assert_eq!(years[11], None, "2200");
        assert_eq!(end.cursor(), 10, "the cursor starts on 2199");

        // A middle decade shows twelve years: the decade and one on each side.
        let mut middle = state_at(2083, 6, 1);
        middle.title_clicked();
        middle.title_clicked();
        let years: Vec<u16> = middle.decade_years().iter().flatten().copied().collect();
        assert_eq!(years, (2079..=2090).collect::<Vec<_>>());
    }

    #[test]
    fn wr03_ac6_stepping_the_decade_moves_the_cursor_off_an_empty_cell() {
        let mut state = state_at(1915, 6, 1);
        state.title_clicked();
        state.title_clicked();
        assert_eq!(state.decade(), 1910);
        state.step(-1);
        assert_eq!(state.decade(), 1900);
        assert!(
            year_under_cursor(state).is_some(),
            "the cursor is on a year, not on the empty 1899 or 1900 cell"
        );
    }

    #[test]
    fn wr03_ac6_escape_steps_decade_to_year_to_month_then_leaves_it_to_the_popup() {
        let mut state = state_at(2083, 7, 5);
        state.title_clicked();
        state.title_clicked();
        assert_eq!(state.zoom(), Zoom::Decade);

        assert!(state.escape(), "decade to year");
        assert_eq!(state.zoom(), Zoom::Year);
        assert!(state.escape(), "year to month");
        assert_eq!(state.zoom(), Zoom::Month);
        assert!(!state.escape(), "the month view's Escape closes the popup");
        assert_eq!(state.zoom(), Zoom::Month);
        assert_eq!(
            state.viewed(),
            (2083, Month::Kartik),
            "Escape never changes the month"
        );
    }

    #[test]
    fn wr03_ac6_today_returns_to_todays_month_from_every_level() {
        let today = date(2083, 7, 5);
        for zoom_clicks in 0..3 {
            let mut state = ZoomState::new(today);
            for _ in 0..zoom_clicks {
                state.title_clicked();
            }
            // Wander off: a far month, then a step or two at the current level.
            state.jump_to(date(2150, 1, 1));
            for _ in 0..zoom_clicks {
                state.title_clicked();
            }
            state.step(1);
            state.step(1);
            state.today(today);
            assert_eq!(state.zoom(), Zoom::Month, "{zoom_clicks} clicks");
            assert_eq!(
                state.viewed(),
                (2083, Month::Kartik),
                "{zoom_clicks} clicks"
            );
            assert_eq!(state.cursor(), 5);
            assert_eq!(state.decade(), 2080);
        }
    }

    #[test]
    fn wr03_ac6_the_today_pill_shows_everywhere_but_todays_month_view() {
        let today = date(2083, 7, 5);
        let mut state = ZoomState::new(today);
        assert!(!state.today_pill_shown(today), "today's month view");

        state.step(1);
        assert!(state.today_pill_shown(today), "another month");
        state.today(today);

        state.title_clicked();
        assert!(
            state.today_pill_shown(today),
            "the year view, even of today's year"
        );
        state.title_clicked();
        assert!(state.today_pill_shown(today), "the decade view");
        state.escape();
        state.escape();
        assert!(!state.today_pill_shown(today), "back on today's month");
    }

    #[test]
    fn wr03_ac6_today_works_at_both_edges_of_the_range() {
        for today in [date(1901, 1, 1), date(2199, 12, 30)] {
            let mut state = ZoomState::new(today);
            state.title_clicked();
            state.title_clicked();
            state.today(today);
            assert_eq!(state.zoom(), Zoom::Month);
            assert_eq!(state.cursor_date(), Some(today));
            assert!(!state.today_pill_shown(today));
        }
    }

    #[test]
    fn wr02_ac2_a_reopen_returns_to_the_month_view_of_the_same_month() {
        let today = date(2083, 7, 5);
        let mut state = ZoomState::new(today);
        state.step(1);
        state.step(1);
        state.title_clicked();
        state.title_clicked();
        state.reopened(today);
        assert_eq!(state.zoom(), Zoom::Month);
        assert_eq!(state.viewed(), (2083, Month::Poush));
        assert_eq!(state.cursor(), 1, "today is not in this month");

        state.today(today);
        state.title_clicked();
        state.reopened(today);
        assert_eq!(state.cursor(), 5, "today is in this month");
    }

    #[test]
    fn jump_to_opens_the_month_of_a_date_with_the_cursor_on_it() {
        let mut state = ZoomState::new(date(2083, 7, 5));
        state.title_clicked();
        state.jump_to(date(2081, 1, 1));
        assert_eq!(state.zoom(), Zoom::Month);
        assert_eq!(state.cursor_date(), Some(date(2081, 1, 1)));
        assert_eq!(state.decade(), 2080);
    }

    #[test]
    fn wr10_ac2_arrows_move_between_days_and_carry_into_the_next_and_previous_month() {
        let mut state = state_at(2083, 7, 5);
        state.move_cursor(Arrow::Right);
        assert_eq!(state.cursor(), 6);
        state.move_cursor(Arrow::Down);
        assert_eq!(state.cursor(), 13);
        state.move_cursor(Arrow::Up);
        state.move_cursor(Arrow::Left);
        assert_eq!(state.cursor(), 5);

        let mut state = state_at(2083, 7, 1);
        state.move_cursor(Arrow::Left);
        assert_eq!(
            state.viewed(),
            (2083, Month::Ashoj),
            "Left from day 1 is the last day of Ashoj"
        );
        assert_eq!(
            state.cursor_date(),
            Some(date(2083, 6, 1).last_day_of_month())
        );

        let last = date(2083, 7, 1).last_day_of_month();
        let mut state = ZoomState::new(last);
        state.move_cursor(Arrow::Right);
        assert_eq!(state.cursor_date(), Some(date(2083, 8, 1)));

        let mut state = state_at(2083, 7, 3);
        state.move_cursor(Arrow::Up);
        assert_eq!(
            state.cursor_date(),
            date(2083, 7, 3).checked_add_days(-7).ok()
        );
    }

    #[test]
    fn wr10_ac2_arrows_stop_at_the_first_and_last_day_of_the_range() {
        let mut start = ZoomState::new(NepaliDate::MIN);
        for arrow in [Arrow::Left, Arrow::Up] {
            start.move_cursor(arrow);
            assert_eq!(start.cursor_date(), Some(NepaliDate::MIN), "{arrow:?}");
        }
        let mut end = ZoomState::new(NepaliDate::MAX);
        for arrow in [Arrow::Right, Arrow::Down] {
            end.move_cursor(arrow);
            assert_eq!(end.cursor_date(), Some(NepaliDate::MAX), "{arrow:?}");
        }
    }

    #[test]
    fn wr10_ac2_arrows_move_between_months_in_a_4_by_3_grid_and_carry_into_the_next_year() {
        let mut state = state_at(2083, 7, 5);
        state.title_clicked();
        assert_eq!(state.cursor(), 6);
        state.move_cursor(Arrow::Right);
        assert_eq!(state.cursor(), 7);
        state.move_cursor(Arrow::Down);
        assert_eq!(state.cursor(), 11);
        state.move_cursor(Arrow::Left);
        assert_eq!(state.cursor(), 10);
        state.move_cursor(Arrow::Up);
        assert_eq!(state.cursor(), 6);

        // Right from Chaitra is Baishakh of the next year; Left from Baishakh goes back.
        let mut state = state_at(2083, 12, 5);
        state.title_clicked();
        state.move_cursor(Arrow::Right);
        assert_eq!((state.year(), state.cursor()), (2084, 0));
        state.move_cursor(Arrow::Left);
        assert_eq!((state.year(), state.cursor()), (2083, 11));
        // Down from the bottom row and Up from the top row keep the column.
        state.move_cursor(Arrow::Down);
        assert_eq!((state.year(), state.cursor()), (2084, 3));
        state.move_cursor(Arrow::Up);
        assert_eq!((state.year(), state.cursor()), (2083, 11));
    }

    #[test]
    fn wr10_ac2_the_month_grid_stops_at_the_first_and_last_year() {
        let mut start = state_at(1901, 1, 1);
        start.title_clicked();
        for arrow in [Arrow::Left, Arrow::Up] {
            start.move_cursor(arrow);
            assert_eq!((start.year(), start.cursor()), (1901, 0), "{arrow:?}");
        }
        let mut end = state_at(2199, 12, 1);
        end.title_clicked();
        for arrow in [Arrow::Right, Arrow::Down] {
            end.move_cursor(arrow);
            assert_eq!((end.year(), end.cursor()), (2199, 11), "{arrow:?}");
        }
    }

    #[test]
    fn wr10_ac2_arrows_move_between_years_and_carry_into_the_next_decade() {
        let mut state = state_at(2083, 7, 5);
        state.title_clicked();
        state.title_clicked();
        assert_eq!(year_under_cursor(state), Some(2083));
        state.move_cursor(Arrow::Right);
        assert_eq!(year_under_cursor(state), Some(2084));
        state.move_cursor(Arrow::Down);
        assert_eq!(year_under_cursor(state), Some(2088));
        state.move_cursor(Arrow::Down);
        assert_eq!(year_under_cursor(state), Some(2092));
        assert_eq!(
            state.decade(),
            2090,
            "the cursor carried into the next decade"
        );
        state.move_cursor(Arrow::Up);
        assert_eq!(year_under_cursor(state), Some(2088));
        state.move_cursor(Arrow::Left);
        assert_eq!(year_under_cursor(state), Some(2087));
        assert_eq!(state.year(), 2083, "the viewed year waits for a pick");
    }

    #[test]
    fn wr10_ac2_the_year_grid_stops_at_1901_and_2199() {
        let mut start = state_at(1901, 1, 1);
        start.title_clicked();
        start.title_clicked();
        for arrow in [Arrow::Left, Arrow::Up] {
            start.move_cursor(arrow);
            assert_eq!(year_under_cursor(start), Some(1901), "{arrow:?}");
        }
        let mut end = state_at(2199, 12, 1);
        end.title_clicked();
        end.title_clicked();
        for arrow in [Arrow::Right, Arrow::Down] {
            end.move_cursor(arrow);
            assert_eq!(year_under_cursor(end), Some(2199), "{arrow:?}");
        }
    }

    /// Whatever sequence of clicks, steps, picks and keys arrives, the state stays inside the
    /// range and the cursor stays on a real cell: a fixed pseudo-random walk from each edge and
    /// the middle.
    #[test]
    fn the_state_stays_valid_under_any_sequence_of_actions() {
        let mut seed = 0x2545_F491_4F6C_DD1D_u64;
        let mut next = move |bound: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % bound
        };
        for start in [date(1901, 1, 1), date(2083, 7, 5), date(2199, 12, 30)] {
            let mut state = ZoomState::new(start);
            for _ in 0..20_000 {
                match next(12) {
                    0 => state.title_clicked(),
                    1 => {
                        state.step(1);
                    }
                    2 => {
                        state.step(-1);
                    }
                    3 => {
                        state.escape();
                    }
                    4 => state.today(start),
                    5 => {
                        state.pick_cursor();
                    }
                    6 => state.move_cursor(Arrow::Left),
                    7 => state.move_cursor(Arrow::Right),
                    8 => state.move_cursor(Arrow::Up),
                    9 => state.move_cursor(Arrow::Down),
                    10 => {
                        let year = u16::try_from(1890 + next(320)).unwrap_or(2000);
                        state.pick_year(year);
                    }
                    _ => state.reopened(start),
                }
                assert!(
                    (FIRST_YEAR..=LAST_YEAR).contains(&state.year()),
                    "year {} after a walk from {start}",
                    state.year()
                );
                assert!((FIRST_DECADE..=LAST_DECADE).contains(&state.decade()));
                assert_eq!(state.decade() % 10, 0);
                match state.zoom() {
                    Zoom::Month => assert!(state.cursor_date().is_some(), "{state:?}"),
                    Zoom::Year => assert!(state.cursor() < CELLS, "{state:?}"),
                    Zoom::Decade => assert!(year_under_cursor(state).is_some(), "{state:?}"),
                }
            }
        }
    }
}
