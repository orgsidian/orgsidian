//! Implements FR-18 (Personal GTD + Student starters; Freelancer + Empty deferred)
//!
//! Content for the **Student** starter, shaped around a term's coursework
//! rhythm rather than GTD's generic life-management categories. Four files,
//! written flat at the Vault root by [`super::generate_starter_vault`]:
//!
//! - `inbox.org` — unprocessed capture (questions for office hours, things
//!   to print, stray reminders) as bare `TODO` items with no dates or
//!   scheduling yet.
//! - `courses.org` — one active course ("Introduction to Statistics") with a
//!   `TODO`/`NEXT`/`DONE` mix of readings/problem sets, `SCHEDULED` for
//!   today and the days after (Today + Week Agenda), a `DEADLINE`, and a
//!   `CLOSED` entry.
//! - `journal.org` — a couple of study-log entries (reflection, not tasks).
//! - `someday.org` — the Someday/Maybe list: courses/topics to explore
//!   later, no states.

use chrono::{Duration, NaiveDate};

use super::{active_timestamp, inactive_timestamp};

/// The Student starter's `.org` files (name, content), relative to `today`.
pub(super) fn files(today: NaiveDate) -> Vec<(&'static str, String)> {
    vec![
        ("inbox.org", inbox()),
        ("courses.org", courses(today)),
        ("journal.org", journal(today)),
        ("someday.org", someday()),
    ]
}

fn inbox() -> String {
    "\
#+TITLE: Inbox
#+STARTUP: showall

Capture anything here the moment it crosses your mind — a question for the
professor, a slide deck to print, a reminder about a study group. Don't
organize yet; that happens during your next weekly planning session, when
each item moves to Courses, Someday/Maybe, or gets done in two minutes and
deleted.

* TODO Ask the professor about extra credit
* TODO Print lecture slides for tomorrow's class
* TODO Find a study group for finals
"
    .to_string()
}

fn courses(today: NaiveDate) -> String {
    let scheduled_today = active_timestamp(today);
    let scheduled_tomorrow = active_timestamp(today + Duration::days(1));
    let scheduled_in_3 = active_timestamp(today + Duration::days(3));
    let deadline_in_4 = active_timestamp(today + Duration::days(4));
    let closed_3_ago = inactive_timestamp(today - Duration::days(3));

    format!(
        "\
#+TITLE: Courses
#+STARTUP: showall

* Introduction to Statistics
Assignments and readings for the one active course this Starter Vault ships
with — add your own courses alongside it as the term goes on.

** DONE Submit problem set 2
   CLOSED: {closed_3_ago}
** TODO Read Chapter 4 — Probability Distributions
   SCHEDULED: {scheduled_today}
** NEXT Problem set 3
   DEADLINE: {deadline_in_4} SCHEDULED: {scheduled_tomorrow}
** TODO Review lecture notes before the midterm
   SCHEDULED: {scheduled_in_3}
"
    )
}

fn journal(today: NaiveDate) -> String {
    let today_ts = inactive_timestamp(today);
    let yesterday_ts = inactive_timestamp(today - Duration::days(1));

    format!(
        "\
#+TITLE: Journal
#+STARTUP: showall

A running study log — what clicked, what didn't, questions to follow up on.
Not part of the Agenda (timestamps here are inactive on purpose).

* {today_ts} Study log
Today's notes go here.

* {yesterday_ts} Study log
Yesterday's notes.
"
    )
}

fn someday() -> String {
    "\
#+TITLE: Someday / Maybe
#+STARTUP: showall

Courses and topics worth exploring but not committing to yet. Bare list on
purpose — no states, no dates. Revisit during a weekly review and promote
anything that's become real (a course you're actually taking) into Courses.

* Look into an elective on data visualization
* Learn touch typing properly over the summer
* Read up on spaced-repetition study techniques
"
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 5).expect("valid date")
    }

    #[test]
    fn files_include_inbox_at_root_and_three_others() {
        let files = files(today());
        assert_eq!(files.len(), 4);
        assert!(files.iter().any(|(name, _)| *name == "inbox.org"));
    }

    #[test]
    fn courses_has_a_headline_scheduled_for_today() {
        let content = courses(today());
        assert!(content.contains(&active_timestamp(today())));
    }

    #[test]
    fn courses_has_a_headline_scheduled_within_the_week() {
        let content = courses(today());
        assert!(content.contains(&active_timestamp(today() + Duration::days(3))));
    }

    #[test]
    fn courses_parses_with_the_expected_todo_shape() {
        let doc = orgsidian_parser::analyze(&courses(today())).expect("parse never fails");
        let course = doc
            .headlines
            .iter()
            .find(|h| h.title.trim() == "Introduction to Statistics")
            .expect("course headline present");
        let items = &course.children;
        assert_eq!(items.len(), 4);

        let scheduled_today = items
            .iter()
            .find(|h| h.scheduled.as_ref().map(|ts| ts.date) == Some(today()))
            .expect("one item scheduled for today");
        assert_eq!(
            scheduled_today
                .todo_state
                .as_ref()
                .map(|s| s.keyword.as_str()),
            Some("TODO")
        );

        // The `NEXT` keyword parses as a distinct TODO state (active keyword in
        // the default sequence), not swallowed as plain title text.
        assert!(
            items
                .iter()
                .any(|h| h.todo_state.as_ref().map(|s| s.keyword.as_str()) == Some("NEXT")),
            "one item is in the NEXT state"
        );

        let done = items
            .iter()
            .find(|h| h.todo_state.as_ref().map(|s| s.keyword.as_str()) == Some("DONE"))
            .expect("one item already DONE");
        // A completed item's CLOSED stamp must be inactive and in the past, and
        // it must not still be SCHEDULED, or it would leak into the agenda.
        let closed = done.closed.as_ref().expect("DONE item carries CLOSED");
        assert!(closed.date < today(), "CLOSED must be in the past");
        assert!(!closed.active, "CLOSED must be an inactive timestamp");
        assert!(done.scheduled.is_none(), "DONE item must not be SCHEDULED");

        // The NEXT problem set carries both stamps with correct, distinct dates
        // (guards the same-line `DEADLINE: <..> SCHEDULED: <..>` planning-line
        // format — a mis-split would swap or duplicate these).
        let deadline_bearer = items
            .iter()
            .find(|h| h.deadline.is_some())
            .expect("one item carries a DEADLINE");
        let sched = deadline_bearer
            .scheduled
            .as_ref()
            .expect("deadline-bearing item is also SCHEDULED");
        let dead = deadline_bearer.deadline.as_ref().expect("has DEADLINE");
        assert_eq!(sched.date, today() + Duration::days(1));
        assert_eq!(dead.date, today() + Duration::days(4));
        assert!(sched.date < dead.date, "SCHEDULED must precede DEADLINE");

        // Every agenda-bearing stamp falls within today..=today+7 (Week Agenda).
        for item in items {
            for ts in [item.scheduled.as_ref(), item.deadline.as_ref()]
                .into_iter()
                .flatten()
            {
                assert!(
                    (today()..=today() + Duration::days(7)).contains(&ts.date),
                    "agenda stamp {} outside the Week Agenda window",
                    ts.date
                );
            }
        }
    }

    #[test]
    fn inbox_items_carry_no_scheduling() {
        let doc = orgsidian_parser::analyze(&inbox()).expect("parse never fails");
        assert!(doc
            .headlines
            .iter()
            .all(|h| h.scheduled.is_none() && h.deadline.is_none() && h.closed.is_none()));
    }

    #[test]
    fn journal_and_someday_stay_out_of_the_agenda() {
        for content in [journal(today()), someday()] {
            let doc = orgsidian_parser::analyze(&content).expect("parse never fails");
            assert!(!doc.headlines.is_empty());
            assert!(
                doc.headlines
                    .iter()
                    .all(|h| h.scheduled.is_none() && h.deadline.is_none()),
                "journal/someday headlines must carry no active planning stamps"
            );
        }
    }
}
