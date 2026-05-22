//! Day-1 anchor test: a noop plugin can be constructed and the trait surface
//! is object-safe under `Box<dyn OrgsidianPlugin>`.
//!
//! Acts as the anti-placebo-green guard (Party Mode P2) for the plugin-api
//! trait surface — fails to compile if a future story sneaks in a generic
//! method, removes a default impl, or otherwise breaks the contract Story
//! 1.5 locks.

use orgsidian_plugin_api::{
    AgendaItem, AgendaQuery, CaptureEntry, Event, HookContext, HookOutcome, OrgsidianPlugin,
    PluginContext, PluginError, PluginMetadata, Result,
};

struct NoopPlugin {
    meta: PluginMetadata,
}

impl OrgsidianPlugin for NoopPlugin {
    fn metadata(&self) -> PluginMetadata {
        self.meta.clone()
    }

    fn init(&mut self, _ctx: &dyn PluginContext) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

struct StubPluginContext {
    meta: PluginMetadata,
}

impl PluginContext for StubPluginContext {
    fn plugin_metadata(&self) -> &PluginMetadata {
        &self.meta
    }
}

struct StubHookContext;

impl HookContext for StubHookContext {
    fn read_vault_file(&self, _path: &str) -> Result<String> {
        Ok(String::new())
    }

    fn query_index(&self, _query: &str) -> Result<String> {
        Ok(String::new())
    }

    fn emit_event(&self, _event: Event) -> Result<()> {
        Ok(())
    }
}

#[test]
fn noop_plugin_is_object_safe_and_defaults_work() {
    let meta = PluginMetadata {
        id: "noop".to_string(),
        name: "Noop".to_string(),
        version: "0.0.0".to_string(),
        author: "tests".to_string(),
    };
    let mut plugin: Box<dyn OrgsidianPlugin> = Box::new(NoopPlugin { meta: meta.clone() });

    assert_eq!(plugin.priority(), 0);
    assert!(plugin.on_event(&Event::IndexRebuilt).is_ok());

    let ctx = StubPluginContext { meta };
    assert!(plugin.init(&ctx).is_ok());

    let hook_ctx = StubHookContext;
    let outcome = plugin
        .on_save_before(&hook_ctx, "content")
        .expect("default impl returns Ok");
    assert!(matches!(outcome, HookOutcome::Continue));

    let entry = CaptureEntry {
        raw_text: "x".into(),
    };
    let outcome = plugin
        .on_capture_before(&hook_ctx, &entry)
        .expect("default impl returns Ok");
    assert!(matches!(outcome, HookOutcome::Continue));

    let query = AgendaQuery {
        raw_filter: String::new(),
    };
    let mut results: Vec<AgendaItem> = Vec::new();
    assert!(plugin
        .on_agenda_query_after(&hook_ctx, &query, &mut results)
        .is_ok());

    assert!(plugin.shutdown().is_ok());
}

#[test]
fn cancel_outcome_carries_reason() {
    let outcome: HookOutcome<String> = HookOutcome::Cancel("plugin foo declined".into());
    match outcome {
        HookOutcome::Cancel(reason) => assert_eq!(reason, "plugin foo declined"),
        other => panic!("expected Cancel, got {other:?}"),
    }
}

#[test]
fn plugin_error_is_display() {
    let err = PluginError::Runtime {
        reason: "boom".into(),
    };
    let rendered = err.to_string();
    assert!(rendered.contains("boom"));
}

#[test]
fn context_traits_are_object_safe() {
    let meta = PluginMetadata {
        id: "noop".to_string(),
        name: "Noop".to_string(),
        version: "0.0.0".to_string(),
        author: "tests".to_string(),
    };
    let _plugin_ctx: Box<dyn PluginContext> = Box::new(StubPluginContext { meta });
    let _hook_ctx: Box<dyn HookContext> = Box::new(StubHookContext);
}

/// Compile-time exhaustive match over every `Event` variant — fails to
/// compile if a future story renames a variant or reshapes a field,
/// locking the day-1 surface per AC3.
#[allow(dead_code)]
fn _event_surface_is_locked(event: Event) {
    match event {
        Event::FileOpened { path: _ } => {}
        Event::FileSaved { path: _ } => {}
        Event::FileChanged { path: _ } => {}
        Event::HeadlineEdited {
            file: _,
            headline_id: _,
        } => {}
        Event::ClockStarted {
            file: _,
            headline_id: _,
        } => {}
        Event::ClockStopped {
            file: _,
            headline_id: _,
        } => {}
        Event::CaptureSubmitted { entry: _ } => {}
        Event::AgendaQueried { query: _ } => {}
        Event::IndexRebuilt => {}
        // `#[non_exhaustive]` requires the wildcard arm — future variants
        // land as SemVer-minor additions per LD-26.
        _ => {}
    }
}

#[test]
fn all_event_variants_construct() {
    let _opened = Event::FileOpened {
        path: "a.org".into(),
    };
    let _saved = Event::FileSaved {
        path: "a.org".into(),
    };
    let _changed = Event::FileChanged {
        path: "a.org".into(),
    };
    let _edited = Event::HeadlineEdited {
        file: "a.org".into(),
        headline_id: "h1".into(),
    };
    let _started = Event::ClockStarted {
        file: "a.org".into(),
        headline_id: "h1".into(),
    };
    let _stopped = Event::ClockStopped {
        file: "a.org".into(),
        headline_id: "h1".into(),
    };
    let _captured = Event::CaptureSubmitted {
        entry: CaptureEntry {
            raw_text: "note".into(),
        },
    };
    let _queried = Event::AgendaQueried {
        query: AgendaQuery {
            raw_filter: String::new(),
        },
    };
    let _rebuilt = Event::IndexRebuilt;
}
