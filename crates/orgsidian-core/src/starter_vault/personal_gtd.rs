//! Implements FR-18 (Personal GTD + Student starters; Freelancer + Empty deferred)
//!
//! Content for the **Personal GTD** starter (David Allen's "Getting Things
//! Done": capture everything into an Inbox, organize into Projects/Next
//! Actions, park the rest on Someday/Maybe, review regularly). Four files,
//! written flat at the Vault root by [`super::generate_starter_vault`]:
//!
//! - `inbox.org` — unprocessed capture as bare `TODO` items with no dates or
//!   scheduling yet (that's the point of an Inbox: capture first, then file
//!   into Projects/Someday during a review).
//! - `projects.org` — one active project ("Repaint the garage") with a
//!   `TODO`/`NEXT`/`DONE` mix of Next Actions, `SCHEDULED` for today and the
//!   days after (Today + Week Agenda), a `DEADLINE`, and a `CLOSED` entry.
//! - `journal.org` — a couple of daily-log entries (reflection, not tasks).
//! - `someday.org` — the Someday/Maybe parking lot: bare ideas, no states.

use chrono::{Duration, NaiveDate};

use super::{active_timestamp, inactive_timestamp};

/// The Personal GTD starter's `.org` files (name, content), relative to
/// `today`.
pub(super) fn files(today: NaiveDate) -> Vec<(&'static str, String)> {
    vec![
        ("inbox.org", inbox()),
        ("projects.org", projects(today)),
        ("journal.org", journal(today)),
        ("someday.org", someday()),
    ]
}

fn inbox() -> String {
    "\
#+TITLE: Inbox
#+STARTUP: showall

Capture anything here the moment it crosses your mind — a call to make, an
idea, a stray errand. Don't organize yet; that happens during your next GTD
review, when each item moves to Projects, Someday/Maybe, or gets done in two
minutes and deleted.

* TODO Call the dentist to reschedule the cleaning
* TODO Look into a birthday gift for Mom
* TODO Return the library book before it's overdue
"
    .to_string()
}

fn projects(today: NaiveDate) -> String {
    let scheduled_today = active_timestamp(today);
    let scheduled_tomorrow = active_timestamp(today + Duration::days(1));
    let scheduled_in_3 = active_timestamp(today + Duration::days(3));
    let deadline_in_5 = active_timestamp(today + Duration::days(5));
    let closed_2_ago = inactive_timestamp(today - Duration::days(2));

    format!(
        "\
#+TITLE: Projects
#+STARTUP: showall

* Repaint the garage
Next Actions for the one active project this Starter Vault ships with —
add your own projects alongside it as they come off the Inbox.

** DONE Clear out the garage and cover the floor
   CLOSED: {closed_2_ago}
** TODO Buy primer and exterior paint
   SCHEDULED: {scheduled_today}
** NEXT Mask the windows and trim
   SCHEDULED: {scheduled_tomorrow}
** TODO Apply the first coat
   DEADLINE: {deadline_in_5} SCHEDULED: {scheduled_in_3}
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

A running daily log — reflections, decisions, things worth remembering. Not
part of the Agenda (timestamps here are inactive on purpose).

* {today_ts} Daily log
Today's notes go here.

* {yesterday_ts} Daily log
Yesterday's notes.
"
    )
}

fn someday() -> String {
    "\
#+TITLE: Someday / Maybe
#+STARTUP: showall

Ideas worth keeping but not committing to yet. Bare list on purpose — no
states, no dates. Revisit during a weekly review and promote anything that's
become real into Projects.

* Learn woodworking basics
* Plan a long weekend trip somewhere new
* Read more about home automation
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
    fn projects_has_a_headline_scheduled_for_today() {
        let content = projects(today());
        assert!(content.contains(&active_timestamp(today())));
    }

    #[test]
    fn projects_has_a_headline_scheduled_within_the_week() {
        let content = projects(today());
        assert!(content.contains(&active_timestamp(today() + Duration::days(3))));
    }

    #[test]
    fn projects_parses_with_the_expected_todo_shape() {
        let doc = orgsidian_parser::analyze(&projects(today())).expect("parse never fails");
        let project = doc
            .headlines
            .iter()
            .find(|h| h.title.trim() == "Repaint the garage")
            .expect("project headline present");
        let actions = &project.children;
        assert_eq!(actions.len(), 4);

        let scheduled_today = actions
            .iter()
            .find(|h| h.scheduled.as_ref().map(|ts| ts.date) == Some(today()))
            .expect("one Next Action scheduled for today");
        assert_eq!(
            scheduled_today
                .todo_state
                .as_ref()
                .map(|s| s.keyword.as_str()),
            Some("TODO")
        );

        // The `NEXT` Next-Action keyword parses as a distinct TODO state (it is
        // an active keyword in the default sequence — guards against it being
        // silently swallowed as plain title text).
        assert!(
            actions
                .iter()
                .any(|h| h.todo_state.as_ref().map(|s| s.keyword.as_str()) == Some("NEXT")),
            "one Next Action is in the NEXT state"
        );

        let done = actions
            .iter()
            .find(|h| h.todo_state.as_ref().map(|s| s.keyword.as_str()) == Some("DONE"))
            .expect("one Next Action already DONE");
        // A completed action's CLOSED stamp must be in the past and it must not
        // still be SCHEDULED, or it would leak back into the agenda.
        let closed = done.closed.as_ref().expect("DONE action carries CLOSED");
        assert!(closed.date < today(), "CLOSED must be in the past");
        assert!(!closed.active, "CLOSED must be an inactive timestamp");
        assert!(
            done.scheduled.is_none(),
            "DONE action must not be SCHEDULED"
        );

        // The combined-planning-line action carries *both* stamps with the
        // correct, distinct dates (guards the same-line `DEADLINE: <..>
        // SCHEDULED: <..>` format the story hinges on — a mis-split would swap
        // or duplicate these).
        let deadline_bearer = actions
            .iter()
            .find(|h| h.deadline.is_some())
            .expect("one Next Action carries a DEADLINE");
        let sched = deadline_bearer
            .scheduled
            .as_ref()
            .expect("deadline-bearing action is also SCHEDULED");
        let dead = deadline_bearer.deadline.as_ref().expect("has DEADLINE");
        assert_eq!(sched.date, today() + Duration::days(3));
        assert_eq!(dead.date, today() + Duration::days(5));
        assert!(sched.date < dead.date, "SCHEDULED must precede DEADLINE");

        // Every agenda-bearing stamp falls within today..=today+7 (Week Agenda).
        for action in actions {
            for ts in [action.scheduled.as_ref(), action.deadline.as_ref()]
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
