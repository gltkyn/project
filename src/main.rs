mod actions;
mod ai;
mod app;
mod domain;
mod events;
mod mod_daily;
mod mod_diary;
mod mod_todo;
mod storage;
mod styles;

use color_eyre::Result;
use serde_json::json;

fn main() -> Result<()> {
    color_eyre::install()?;

    let home = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let data_dir = home.join("YZ").join("data");
    std::fs::create_dir_all(&data_dir)?;
    std::env::set_current_dir(&data_dir)?;

    events::init_event_log();
    events::log_event(json!({
        "type": "boot",
        "app": "ratatui-persist-two-tabs",
        "data_dir": data_dir.display().to_string(),
    }));

    if let Err(e) = storage::check_versions() {
        events::log_event(json!({"type":"fatal","reason":"version-too-new","error":e}));
        return Err(color_eyre::eyre::eyre!(
            "{e}\n\n\
             The data files were created by a newer version of this app.\n\
             Upgrade this app, or back up & remove the offending file to proceed."
        ));
    }
    if let Err(e) = storage::migrate_all() {
        events::log_event(json!({"type":"migration-failed","error":e}));
        eprintln!("Warning: migration failed: {e}");
    }

    let app = app::App::new();
    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal);
    ratatui::restore();

    events::log_event(json!({"type":"session-end"}));
    result
}

#[cfg(test)]
mod tests {
    use crate::domain::{Message, Status, TodoItem};
    use crate::events::{parse_event_id, parse_id_num};
    use crate::storage::{
        detect_messages_file_version, migrate_counters_str, migrate_messages_str,
        migrate_todos_str, MigrateCountersResult, MigrateMessagesResult, MigrateTodosResult,
        VersionedCounters, VersionedTodos, COUNTERS_VERSION, TODOS_VERSION,
    };
    use crate::styles::{alternate_colors, display_width, ALT_ROW_BG_COLOR, NORMAL_ROW_BG};

    #[test]
    fn parse_event_id_works() {
        assert_eq!(parse_event_id("[2026-01-01 00:00:00.000] evt-42 | {}"), Some(42));
        assert_eq!(parse_event_id("garbage"), None);
        assert_eq!(parse_event_id("[ts] evt- | {}"), None);
    }

    #[test]
    fn parse_id_num_works() {
        assert_eq!(parse_id_num("todo-7", "todo-"), Some(7));
        assert_eq!(parse_id_num("msg-12", "msg-"), Some(12));
        assert_eq!(parse_id_num("msg-12", "todo-"), None);
        assert_eq!(parse_id_num("todo-abc", "todo-"), None);
        assert_eq!(parse_id_num("", "todo-"), None);
    }

    #[test]
    fn message_from_line_validates_prefix() {
        let line = "msg-1 | 2026-01-01 00:00:00 | hello | world";
        let m = Message::from_line(line).unwrap();
        assert_eq!(m.id, "msg-1");
        assert_eq!(m.timestamp, "2026-01-01 00:00:00");
        assert_eq!(m.content, "hello | world");

        assert!(Message::from_line("other-1 | ts | hi").is_none());
        assert!(Message::from_line("# version: 1").is_none());
        assert!(Message::from_line("no pipes here").is_none());
    }

    #[test]
    fn message_roundtrip() {
        let m = Message::new("msg-5".into(), "hi there".into());
        let back = Message::from_line(&m.to_line()).unwrap();
        assert_eq!(m.id, back.id);
        assert_eq!(m.timestamp, back.timestamp);
        assert_eq!(m.content, back.content);
    }

    #[test]
    fn detect_messages_version() {
        assert_eq!(detect_messages_file_version("# version: 1\nmsg-1 | ts | x\n"), 1);
        assert_eq!(detect_messages_file_version("# version: 2\n"), 2);
        assert_eq!(detect_messages_file_version("msg-1 | ts | x\n"), 0);
        assert_eq!(detect_messages_file_version(""), 0);
    }

    #[test]
    fn migrate_todos_v1_no_change() {
        let json = r#"{"version":1,"items":[]}"#;
        assert!(matches!(migrate_todos_str(json), MigrateTodosResult::NoChange));
    }

    #[test]
    fn migrate_todos_v0_array() {
        let json = r#"[{"id":"todo-1","status":"Todo","todo":"x","info":""}]"#;
        match migrate_todos_str(json) {
            MigrateTodosResult::Migrated { new_content, count } => {
                assert_eq!(count, 1);
                let vt: VersionedTodos = serde_json::from_str(&new_content).unwrap();
                assert_eq!(vt.version, TODOS_VERSION);
                assert_eq!(vt.items.len(), 1);
                assert_eq!(vt.items[0].id, "todo-1");
            }
            other => panic!("expected Migrated, got {other:?}"),
        }
    }

    #[test]
    fn migrate_todos_empty_and_corrupt() {
        assert!(matches!(migrate_todos_str(""), MigrateTodosResult::NoChange));
        assert!(matches!(migrate_todos_str("   \n"), MigrateTodosResult::NoChange));
        assert!(matches!(migrate_todos_str("not json"), MigrateTodosResult::Corrupt));
    }

    #[test]
    fn migrate_counters_v1_no_change() {
        let json = r#"{"version":1,"next_todo":5,"next_msg":6,"next_evt":7}"#;
        assert!(matches!(migrate_counters_str(json), MigrateCountersResult::NoChange));
    }

    #[test]
    fn migrate_counters_v0_legacy() {
        let json = r#"{"next_todo":5,"next_msg":6,"next_evt":7}"#;
        match migrate_counters_str(json) {
            MigrateCountersResult::Migrated { new_content } => {
                let vc: VersionedCounters = serde_json::from_str(&new_content).unwrap();
                assert_eq!(vc.version, COUNTERS_VERSION);
                assert_eq!(vc.next_todo, 5);
                assert_eq!(vc.next_msg, 6);
                assert_eq!(vc.next_evt, 7);
            }
            other => panic!("expected Migrated, got {other:?}"),
        }
    }

    #[test]
    fn migrate_counters_corrupt() {
        assert!(matches!(migrate_counters_str("nonsense"), MigrateCountersResult::Corrupt));
    }

    #[test]
    fn migrate_messages_v1_no_change() {
        let content = "# version: 1\nmsg-1 | ts | hi\n";
        assert!(matches!(migrate_messages_str(content), MigrateMessagesResult::NoChange));
    }

    #[test]
    fn migrate_messages_v0_adds_header() {
        let content = "msg-1 | ts | hi\n";
        match migrate_messages_str(content) {
            MigrateMessagesResult::Migrated(s) => {
                assert!(s.starts_with("# version: "));
                assert!(s.ends_with("msg-1 | ts | hi\n"));
            }
            other => panic!("expected Migrated, got {other:?}"),
        }
    }

    #[test]
    fn migrate_messages_empty_writes_header() {
        assert!(matches!(migrate_messages_str(""), MigrateMessagesResult::WriteHeaderOnly));
        assert!(matches!(migrate_messages_str("   \n"), MigrateMessagesResult::WriteHeaderOnly));
    }

    #[test]
    fn versioned_todos_serialization_roundtrip() {
        let vt = VersionedTodos {
            version: TODOS_VERSION,
            items: vec![TodoItem::new("todo-1".into(), Status::Completed, "a", "b")],
        };
        let json = serde_json::to_string(&vt).unwrap();
        let back: VersionedTodos = serde_json::from_str(&json).unwrap();
        assert_eq!(back.version, TODOS_VERSION);
        assert_eq!(back.items.len(), 1);
        assert_eq!(back.items[0].status, Status::Completed);
        assert_eq!(back.items[0].todo, "a");
    }

    #[test]
    fn future_version_is_detected() {
        let vt = serde_json::from_str::<VersionedTodos>(r#"{"version":99,"items":[]}"#).unwrap();
        assert!(vt.version > TODOS_VERSION);
        let vc = serde_json::from_str::<VersionedCounters>(
            r#"{"version":99,"next_todo":1,"next_msg":1,"next_evt":1}"#,
        ).unwrap();
        assert!(vc.version > COUNTERS_VERSION);
    }

    #[test]
    fn display_width_handles_cjk() {
        assert_eq!(display_width("abc"), 3);
        assert_eq!(display_width("你好"), 4);
        assert_eq!(display_width(""), 0);
    }

    #[test]
    fn alternate_colors_alternates() {
        assert_eq!(alternate_colors(0), NORMAL_ROW_BG);
        assert_eq!(alternate_colors(1), ALT_ROW_BG_COLOR);
        assert_eq!(alternate_colors(2), NORMAL_ROW_BG);
    }
}
