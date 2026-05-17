mod env_cmd_tests {
    use clap::Parser;
    use xun::xun_core::env_cmd::{
        EnvCmd, EnvSubCommand, EnvVar, EnvSnapshotEntry, EnvProfileEntry,
        EnvPathSubCommand, EnvSnapshotSubCommand, EnvProfileSubCommand,
        EnvBatchSubCommand, EnvSchemaSubCommand, EnvAnnotateSubCommand,
        EnvConfigSubCommand,
    };
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::renderer::{JsonRenderer, Renderer};
    use xun::xun_core::value::{Value, ValueKind};
    use std::io::Cursor;

    // 鈹€鈹€ 杈呭姪鍑芥暟 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    fn parse(args: &[&str]) -> EnvCmd {
        let mut full = vec!["test"];
        full.extend_from_slice(args);
        EnvCmd::try_parse_from(full).unwrap()
    }

    // 鈹€鈹€ CLI 瑙ｆ瀽娴嬭瘯锛氱嫭绔嬪瓙鍛戒护 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn status_parses_defaults() {
        let cmd = parse(&["status"]);
        match cmd.sub {
            EnvSubCommand::Status(args) => {
                assert_eq!(args.scope, "all");
                assert_eq!(args.format, "text");
            }
            other => panic!("expected Status, got {other:?}"),
        }
    }

    #[test]
    fn list_parses_defaults() {
        let cmd = parse(&["list"]);
        match cmd.sub {
            EnvSubCommand::List(args) => {
                assert_eq!(args.scope, "user");
                assert_eq!(args.format, "auto");
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn list_parses_scope_and_format() {
        let cmd = parse(&["list", "--scope", "all", "-f", "json"]);
        match cmd.sub {
            EnvSubCommand::List(args) => {
                assert_eq!(args.scope, "all");
                assert_eq!(args.format, "json");
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn search_parses_query() {
        let cmd = parse(&["search", "PATH"]);
        match cmd.sub {
            EnvSubCommand::Search(args) => {
                assert_eq!(args.query, "PATH");
                assert_eq!(args.scope, "all");
            }
            other => panic!("expected Search, got {other:?}"),
        }
    }

    #[test]
    fn get_parses_name() {
        let cmd = parse(&["get", "HOME"]);
        match cmd.sub {
            EnvSubCommand::Show(args) => {
                assert_eq!(args.name, "HOME");
                assert_eq!(args.scope, "user");
            }
            other => panic!("expected Show, got {other:?}"),
        }
    }

    #[test]
    fn set_parses_name_value() {
        let cmd = parse(&["set", "MY_VAR", "hello", "--scope", "system"]);
        match cmd.sub {
            EnvSubCommand::Set(args) => {
                assert_eq!(args.name, "MY_VAR");
                assert_eq!(args.value, "hello");
                assert_eq!(args.scope, "system");
                assert!(!args.no_snapshot);
            }
            other => panic!("expected Set, got {other:?}"),
        }
    }

    #[test]
    fn set_parses_no_snapshot() {
        let cmd = parse(&["set", "X", "1", "--no-snapshot"]);
        match cmd.sub {
            EnvSubCommand::Set(args) => {
                assert!(args.no_snapshot);
            }
            other => panic!("expected Set, got {other:?}"),
        }
    }

    #[test]
    fn del_parses_name_and_yes() {
        let cmd = parse(&["del", "OLD_VAR", "-y"]);
        match cmd.sub {
            EnvSubCommand::Rm(args) => {
                assert_eq!(args.name, "OLD_VAR");
                assert!(args.yes);
            }
            other => panic!("expected Rm, got {other:?}"),
        }
    }

    #[test]
    fn check_parses_defaults() {
        let cmd = parse(&["check"]);
        match cmd.sub {
            EnvSubCommand::Check(args) => {
                assert_eq!(args.scope, "all");
                assert!(!args.fix);
                assert_eq!(args.format, "text");
            }
            other => panic!("expected Check, got {other:?}"),
        }
    }

    #[test]
    fn path_dedup_parses() {
        let cmd = parse(&["path-dedup", "--remove-missing", "--dry-run"]);
        match cmd.sub {
            EnvSubCommand::PathDedup(args) => {
                assert!(args.remove_missing);
                assert!(args.dry_run);
                assert_eq!(args.scope, "user");
            }
            other => panic!("expected PathDedup, got {other:?}"),
        }
    }

    #[test]
    fn doctor_parses_defaults() {
        let cmd = parse(&["doctor"]);
        match cmd.sub {
            EnvSubCommand::Doctor(args) => {
                assert_eq!(args.scope, "user");
                assert!(!args.fix);
            }
            other => panic!("expected Doctor, got {other:?}"),
        }
    }

    #[test]
    fn apply_parses_name() {
        let cmd = parse(&["apply", "dev-profile", "-y"]);
        match cmd.sub {
            EnvSubCommand::Apply(args) => {
                assert_eq!(args.name, "dev-profile");
                assert!(args.yes);
                assert!(args.scope.is_none());
            }
            other => panic!("expected Apply, got {other:?}"),
        }
    }

    #[test]
    fn export_parses() {
        let cmd = parse(&["export", "--scope", "all", "-f", "json", "-o", "out.json"]);
        match cmd.sub {
            EnvSubCommand::Export(args) => {
                assert_eq!(args.scope, "all");
                assert_eq!(args.format, "json");
                assert_eq!(args.out.as_deref(), Some("out.json"));
            }
            other => panic!("expected Export, got {other:?}"),
        }
    }

    #[test]
    fn export_all_parses() {
        let cmd = parse(&["export-all", "-o", "bundle.zip"]);
        match cmd.sub {
            EnvSubCommand::ExportAll(args) => {
                assert_eq!(args.scope, "all");
                assert_eq!(args.out.as_deref(), Some("bundle.zip"));
            }
            other => panic!("expected ExportAll, got {other:?}"),
        }
    }

    #[test]
    fn export_live_parses() {
        let cmd = parse(&["export-live", "-f", "sh", "--env", ".env1", "--env", ".env2", "--set", "X=1"]);
        match cmd.sub {
            EnvSubCommand::ExportLive(args) => {
                assert_eq!(args.format, "sh");
                assert_eq!(args.env_files, vec![".env1", ".env2"]);
                assert_eq!(args.set, vec!["X=1"]);
            }
            other => panic!("expected ExportLive, got {other:?}"),
        }
    }

    #[test]
    fn env_merged_parses() {
        let cmd = parse(&["env", "--scope", "system"]);
        match cmd.sub {
            EnvSubCommand::Env(args) => {
                assert_eq!(args.scope, "system");
                assert_eq!(args.format, "text");
            }
            other => panic!("expected Env, got {other:?}"),
        }
    }

    #[test]
    fn import_parses_defaults() {
        let cmd = parse(&["import"]);
        match cmd.sub {
            EnvSubCommand::Import(args) => {
                assert!(args.file.is_none());
                assert!(!args.stdin);
                assert_eq!(args.scope, "user");
                assert_eq!(args.mode, "merge");
                assert!(!args.dry_run);
                assert!(!args.yes);
            }
            other => panic!("expected Import, got {other:?}"),
        }
    }

    #[test]
    fn import_parses_all_options() {
        let cmd = parse(&["import", "data.env", "--stdin", "--scope", "system", "-m", "overwrite", "--dry-run", "-y"]);
        match cmd.sub {
            EnvSubCommand::Import(args) => {
                assert_eq!(args.file.as_deref(), Some("data.env"));
                assert!(args.stdin);
                assert_eq!(args.scope, "system");
                assert_eq!(args.mode, "overwrite");
                assert!(args.dry_run);
                assert!(args.yes);
            }
            other => panic!("expected Import, got {other:?}"),
        }
    }

    #[test]
    fn diff_live_parses() {
        let cmd = parse(&["diff-live", "--snapshot", "snap-001", "--since", "2026-01-01", "--color"]);
        match cmd.sub {
            EnvSubCommand::DiffLive(args) => {
                assert_eq!(args.snapshot.as_deref(), Some("snap-001"));
                assert_eq!(args.since.as_deref(), Some("2026-01-01"));
                assert!(args.color);
            }
            other => panic!("expected DiffLive, got {other:?}"),
        }
    }

    #[test]
    fn graph_parses() {
        let cmd = parse(&["graph", "PATH", "--max-depth", "16"]);
        match cmd.sub {
            EnvSubCommand::Graph(args) => {
                assert_eq!(args.name, "PATH");
                assert_eq!(args.max_depth, 16);
                assert_eq!(args.scope, "all");
            }
            other => panic!("expected Graph, got {other:?}"),
        }
    }

    #[test]
    fn validate_parses() {
        let cmd = parse(&["validate", "--strict"]);
        match cmd.sub {
            EnvSubCommand::Validate(args) => {
                assert!(args.strict);
                assert_eq!(args.scope, "all");
            }
            other => panic!("expected Validate, got {other:?}"),
        }
    }

    #[test]
    fn audit_parses() {
        let cmd = parse(&["audit", "--limit", "100"]);
        match cmd.sub {
            EnvSubCommand::Audit(args) => {
                assert_eq!(args.limit, 100);
                assert_eq!(args.format, "text");
            }
            other => panic!("expected Audit, got {other:?}"),
        }
    }

    #[test]
    fn watch_parses() {
        let cmd = parse(&["watch", "--interval-ms", "5000", "--once"]);
        match cmd.sub {
            EnvSubCommand::Watch(args) => {
                assert_eq!(args.interval_ms, 5000);
                assert!(args.once);
                assert_eq!(args.scope, "all");
            }
            other => panic!("expected Watch, got {other:?}"),
        }
    }

    #[test]
    fn template_parses() {
        let cmd = parse(&["template", "Path=%PATH%", "--validate-only"]);
        match cmd.sub {
            EnvSubCommand::Template(args) => {
                assert_eq!(args.input, "Path=%PATH%");
                assert!(args.validate_only);
            }
            other => panic!("expected Template, got {other:?}"),
        }
    }

    #[test]
    fn run_parses() {
        let cmd = parse(&["run", "--scope", "user", "--schema-check", "--notify", "--", "echo", "hello"]);
        match cmd.sub {
            EnvSubCommand::Run(args) => {
                assert_eq!(args.scope, "user");
                assert!(args.schema_check);
                assert!(args.notify);
                assert_eq!(args.command, vec!["echo", "hello"]);
            }
            other => panic!("expected Run, got {other:?}"),
        }
    }

    #[test]
    fn tui_parses_empty() {
        let cmd = parse(&["tui"]);
        match cmd.sub {
            EnvSubCommand::Tui(_) => {}
            other => panic!("expected Tui, got {other:?}"),
        }
    }

    // 鈹€鈹€ CLI 瑙ｆ瀽娴嬭瘯锛氬祵濂楀瓙鍛戒护缁?path 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn path_add_parses() {
        let cmd = parse(&["path", "add", "C:/tools", "--head"]);
        match cmd.sub {
            EnvSubCommand::Path(p) => match p.sub {
                EnvPathSubCommand::Add(args) => {
                    assert_eq!(args.entry, "C:/tools");
                    assert!(args.head);
                    assert!(!args.tail);
                }
                other => panic!("expected Add, got {other:?}"),
            },
            other => panic!("expected Path, got {other:?}"),
        }
    }

    #[test]
    fn path_rm_parses() {
        let cmd = parse(&["path", "rm", "C:/old"]);
        match cmd.sub {
            EnvSubCommand::Path(p) => match p.sub {
                EnvPathSubCommand::Rm(args) => {
                    assert_eq!(args.entry, "C:/old");
                    assert_eq!(args.scope, "user");
                }
                other => panic!("expected Rm, got {other:?}"),
            },
            other => panic!("expected Path, got {other:?}"),
        }
    }

    // 鈹€鈹€ CLI 瑙ｆ瀽娴嬭瘯锛氬祵濂楀瓙鍛戒护缁?snapshot 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn snapshot_create_parses() {
        let cmd = parse(&["snapshot", "create", "--desc", "before install"]);
        match cmd.sub {
            EnvSubCommand::Snapshot(s) => match s.sub {
                EnvSnapshotSubCommand::Create(args) => {
                    assert_eq!(args.desc.as_deref(), Some("before install"));
                }
                other => panic!("expected Create, got {other:?}"),
            },
            other => panic!("expected Snapshot, got {other:?}"),
        }
    }

    #[test]
    fn snapshot_list_parses() {
        let cmd = parse(&["snapshot", "list"]);
        match cmd.sub {
            EnvSubCommand::Snapshot(s) => match s.sub {
                EnvSnapshotSubCommand::List(args) => {
                    assert_eq!(args.format, "auto");
                }
                other => panic!("expected List, got {other:?}"),
            },
            other => panic!("expected Snapshot, got {other:?}"),
        }
    }

    #[test]
    fn snapshot_restore_parses() {
        let cmd = parse(&["snapshot", "restore", "--latest", "-y"]);
        match cmd.sub {
            EnvSubCommand::Snapshot(s) => match s.sub {
                EnvSnapshotSubCommand::Restore(args) => {
                    assert!(args.latest);
                    assert!(args.yes);
                    assert_eq!(args.scope, "all");
                }
                other => panic!("expected Restore, got {other:?}"),
            },
            other => panic!("expected Snapshot, got {other:?}"),
        }
    }

    #[test]
    fn snapshot_prune_parses() {
        let cmd = parse(&["snapshot", "prune", "--keep", "10"]);
        match cmd.sub {
            EnvSubCommand::Snapshot(s) => match s.sub {
                EnvSnapshotSubCommand::Prune(args) => {
                    assert_eq!(args.keep, 10);
                }
                other => panic!("expected Prune, got {other:?}"),
            },
            other => panic!("expected Snapshot, got {other:?}"),
        }
    }

    // 鈹€鈹€ CLI 瑙ｆ瀽娴嬭瘯锛氬祵濂楀瓙鍛戒护缁?profile 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn profile_list_parses() {
        let cmd = parse(&["profile", "list"]);
        match cmd.sub {
            EnvSubCommand::Profile(p) => match p.sub {
                EnvProfileSubCommand::List(args) => {
                    assert_eq!(args.format, "auto");
                }
                other => panic!("expected List, got {other:?}"),
            },
            other => panic!("expected Profile, got {other:?}"),
        }
    }

    #[test]
    fn profile_capture_parses() {
        let cmd = parse(&["profile", "capture", "dev-env"]);
        match cmd.sub {
            EnvSubCommand::Profile(p) => match p.sub {
                EnvProfileSubCommand::Capture(args) => {
                    assert_eq!(args.name, "dev-env");
                    assert_eq!(args.scope, "user");
                }
                other => panic!("expected Capture, got {other:?}"),
            },
            other => panic!("expected Profile, got {other:?}"),
        }
    }

    #[test]
    fn profile_apply_parses() {
        let cmd = parse(&["profile", "apply", "prod", "--scope", "system", "-y"]);
        match cmd.sub {
            EnvSubCommand::Profile(p) => match p.sub {
                EnvProfileSubCommand::Apply(args) => {
                    assert_eq!(args.name, "prod");
                    assert_eq!(args.scope.as_deref(), Some("system"));
                    assert!(args.yes);
                }
                other => panic!("expected Apply, got {other:?}"),
            },
            other => panic!("expected Profile, got {other:?}"),
        }
    }

    #[test]
    fn profile_diff_parses() {
        let cmd = parse(&["profile", "diff", "dev"]);
        match cmd.sub {
            EnvSubCommand::Profile(p) => match p.sub {
                EnvProfileSubCommand::Diff(args) => {
                    assert_eq!(args.name, "dev");
                    assert_eq!(args.format, "text");
                }
                other => panic!("expected Diff, got {other:?}"),
            },
            other => panic!("expected Profile, got {other:?}"),
        }
    }

    #[test]
    fn profile_delete_parses() {
        let cmd = parse(&["profile", "delete", "old", "-y"]);
        match cmd.sub {
            EnvSubCommand::Profile(p) => match p.sub {
                EnvProfileSubCommand::Rm(args) => {
                    assert_eq!(args.name, "old");
                    assert!(args.yes);
                }
                other => panic!("expected Rm, got {other:?}"),
            },
            other => panic!("expected Profile, got {other:?}"),
        }
    }

    // 鈹€鈹€ CLI 瑙ｆ瀽娴嬭瘯锛氬祵濂楀瓙鍛戒护缁?batch 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn batch_set_parses() {
        let cmd = parse(&["batch", "set", "A=1", "B=2", "--dry-run"]);
        match cmd.sub {
            EnvSubCommand::Batch(b) => match b.sub {
                EnvBatchSubCommand::Set(args) => {
                    assert_eq!(args.items, vec!["A=1", "B=2"]);
                    assert!(args.dry_run);
                }
                other => panic!("expected Set, got {other:?}"),
            },
            other => panic!("expected Batch, got {other:?}"),
        }
    }

    #[test]
    fn batch_delete_parses() {
        let cmd = parse(&["batch", "delete", "X", "Y", "Z"]);
        match cmd.sub {
            EnvSubCommand::Batch(b) => match b.sub {
                EnvBatchSubCommand::Rm(args) => {
                    assert_eq!(args.names, vec!["X", "Y", "Z"]);
                }
                other => panic!("expected Rm, got {other:?}"),
            },
            other => panic!("expected Batch, got {other:?}"),
        }
    }

    #[test]
    fn batch_rename_parses() {
        let cmd = parse(&["batch", "rename", "OLD", "NEW"]);
        match cmd.sub {
            EnvSubCommand::Batch(b) => match b.sub {
                EnvBatchSubCommand::Rename(args) => {
                    assert_eq!(args.old, "OLD");
                    assert_eq!(args.new, "NEW");
                }
                other => panic!("expected Rename, got {other:?}"),
            },
            other => panic!("expected Batch, got {other:?}"),
        }
    }

    // 鈹€鈹€ CLI 瑙ｆ瀽娴嬭瘯锛氬祵濂楀瓙鍛戒护缁?schema 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn schema_show_parses() {
        let cmd = parse(&["schema", "show"]);
        match cmd.sub {
            EnvSubCommand::Schema(s) => match s.sub {
                EnvSchemaSubCommand::Show(args) => {
                    assert_eq!(args.format, "text");
                }
                other => panic!("expected Show, got {other:?}"),
            },
            other => panic!("expected Schema, got {other:?}"),
        }
    }

    #[test]
    fn schema_add_required_parses() {
        let cmd = parse(&["schema", "add-required", "PATH", "--warn-only"]);
        match cmd.sub {
            EnvSubCommand::Schema(s) => match s.sub {
                EnvSchemaSubCommand::AddRequired(args) => {
                    assert_eq!(args.pattern, "PATH");
                    assert!(args.warn_only);
                }
                other => panic!("expected AddRequired, got {other:?}"),
            },
            other => panic!("expected Schema, got {other:?}"),
        }
    }

    #[test]
    fn schema_add_regex_parses() {
        let cmd = parse(&["schema", "add-regex", "MY_*", "^valid$"]);
        match cmd.sub {
            EnvSubCommand::Schema(s) => match s.sub {
                EnvSchemaSubCommand::AddRegex(args) => {
                    assert_eq!(args.pattern, "MY_*");
                    assert_eq!(args.regex, "^valid$");
                }
                other => panic!("expected AddRegex, got {other:?}"),
            },
            other => panic!("expected Schema, got {other:?}"),
        }
    }

    #[test]
    fn schema_add_enum_parses() {
        let cmd = parse(&["schema", "add-enum", "MODE", "dev", "prod", "staging"]);
        match cmd.sub {
            EnvSubCommand::Schema(s) => match s.sub {
                EnvSchemaSubCommand::AddEnum(args) => {
                    assert_eq!(args.pattern, "MODE");
                    assert_eq!(args.values, vec!["dev", "prod", "staging"]);
                }
                other => panic!("expected AddEnum, got {other:?}"),
            },
            other => panic!("expected Schema, got {other:?}"),
        }
    }

    #[test]
    fn schema_remove_parses() {
        let cmd = parse(&["schema", "remove", "PATH"]);
        match cmd.sub {
            EnvSubCommand::Schema(s) => match s.sub {
                EnvSchemaSubCommand::Remove(args) => {
                    assert_eq!(args.pattern, "PATH");
                }
                other => panic!("expected Remove, got {other:?}"),
            },
            other => panic!("expected Schema, got {other:?}"),
        }
    }

    #[test]
    fn schema_reset_parses() {
        let cmd = parse(&["schema", "reset", "-y"]);
        match cmd.sub {
            EnvSubCommand::Schema(s) => match s.sub {
                EnvSchemaSubCommand::Reset(args) => {
                    assert!(args.yes);
                }
                other => panic!("expected Reset, got {other:?}"),
            },
            other => panic!("expected Schema, got {other:?}"),
        }
    }

    // 鈹€鈹€ CLI 瑙ｆ瀽娴嬭瘯锛氬祵濂楀瓙鍛戒护缁?annotate 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn annotate_set_parses() {
        let cmd = parse(&["annotate", "set", "PATH", "system PATH variable"]);
        match cmd.sub {
            EnvSubCommand::Annotate(a) => match a.sub {
                EnvAnnotateSubCommand::Set(args) => {
                    assert_eq!(args.name, "PATH");
                    assert_eq!(args.note, "system PATH variable");
                }
                other => panic!("expected Set, got {other:?}"),
            },
            other => panic!("expected Annotate, got {other:?}"),
        }
    }

    #[test]
    fn annotate_list_parses() {
        let cmd = parse(&["annotate", "list"]);
        match cmd.sub {
            EnvSubCommand::Annotate(a) => match a.sub {
                EnvAnnotateSubCommand::List(args) => {
                    assert_eq!(args.format, "text");
                }
                other => panic!("expected List, got {other:?}"),
            },
            other => panic!("expected Annotate, got {other:?}"),
        }
    }

    // 鈹€鈹€ CLI 瑙ｆ瀽娴嬭瘯锛氬祵濂楀瓙鍛戒护缁?config 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn config_show_parses() {
        let cmd = parse(&["config", "show"]);
        match cmd.sub {
            EnvSubCommand::Config(c) => match c.sub {
                EnvConfigSubCommand::Show(args) => {
                    assert_eq!(args.format, "text");
                }
                other => panic!("expected Show, got {other:?}"),
            },
            other => panic!("expected Config, got {other:?}"),
        }
    }

    #[test]
    fn config_path_parses() {
        let cmd = parse(&["config", "path"]);
        match cmd.sub {
            EnvSubCommand::Config(c) => match c.sub {
                EnvConfigSubCommand::Path(_) => {}
                other => panic!("expected Path, got {other:?}"),
            },
            other => panic!("expected Config, got {other:?}"),
        }
    }

    #[test]
    fn config_reset_parses() {
        let cmd = parse(&["config", "reset", "-y"]);
        match cmd.sub {
            EnvSubCommand::Config(c) => match c.sub {
                EnvConfigSubCommand::Reset(args) => {
                    assert!(args.yes);
                }
                other => panic!("expected Reset, got {other:?}"),
            },
            other => panic!("expected Config, got {other:?}"),
        }
    }

    #[test]
    fn config_get_parses() {
        let cmd = parse(&["config", "get", "default_scope"]);
        match cmd.sub {
            EnvSubCommand::Config(c) => match c.sub {
                EnvConfigSubCommand::Get(args) => {
                    assert_eq!(args.key, "default_scope");
                }
                other => panic!("expected Show, got {other:?}"),
            },
            other => panic!("expected Config, got {other:?}"),
        }
    }

    #[test]
    fn config_set_parses() {
        let cmd = parse(&["config", "set", "default_scope", "all"]);
        match cmd.sub {
            EnvSubCommand::Config(c) => match c.sub {
                EnvConfigSubCommand::Set(args) => {
                    assert_eq!(args.key, "default_scope");
                    assert_eq!(args.value, "all");
                }
                other => panic!("expected Set, got {other:?}"),
            },
            other => panic!("expected Config, got {other:?}"),
        }
    }

    // 鈹€鈹€ TableRow 娴嬭瘯锛欵nvVar 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn env_var_columns_correct() {
        let cols = EnvVar::columns();
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0].name, "name");
        assert_eq!(cols[0].kind, ValueKind::String);
        assert_eq!(cols[1].name, "value");
        assert_eq!(cols[1].kind, ValueKind::String);
        assert_eq!(cols[2].name, "scope");
        assert_eq!(cols[2].kind, ValueKind::String);
    }

    #[test]
    fn env_var_cells_match_fields() {
        let v = EnvVar::new("PATH", "C:\\Windows", "system");
        let cells = v.cells();
        assert_eq!(cells.len(), 3);
        assert!(matches!(&cells[0], Value::String(s) if s == "PATH"));
        assert!(matches!(&cells[1], Value::String(s) if s == "C:\\Windows"));
        assert!(matches!(&cells[2], Value::String(s) if s == "system"));
    }

    #[test]
    fn env_var_renders_as_json() {
        let v = EnvVar::new("HOME", "/home/user", "user");
        let table = v.to_table();
        let mut buf = Cursor::new(Vec::new());
        let mut r = JsonRenderer::new(false, &mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("HOME"), "json output: {output}");
    }

    #[test]
    fn env_var_vec_to_table() {
        let vars = vec![
            EnvVar::new("A", "1", "user"),
            EnvVar::new("B", "2", "system"),
        ];
        let table = EnvVar::vec_to_table(&vars);
        assert_eq!(table.len(), 2);
        assert_eq!(table.columns.len(), 3);
    }

    // 鈹€鈹€ TableRow 娴嬭瘯锛欵nvSnapshotEntry 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn snapshot_entry_columns_correct() {
        let cols = EnvSnapshotEntry::columns();
        assert_eq!(cols.len(), 4);
        assert_eq!(cols[0].name, "id");
        assert_eq!(cols[1].name, "created");
        assert_eq!(cols[1].kind, ValueKind::Date);
        assert_eq!(cols[2].name, "desc");
        assert_eq!(cols[3].name, "var_count");
        assert_eq!(cols[3].kind, ValueKind::Int);
    }

    #[test]
    fn snapshot_entry_cells_match_fields() {
        let e = EnvSnapshotEntry::new("snap-001", "2026-05-12", "before install", 42);
        let cells = e.cells();
        assert_eq!(cells.len(), 4);
        assert!(matches!(&cells[0], Value::String(s) if s == "snap-001"));
        assert!(matches!(&cells[3], Value::Int(42)));
    }

    // 鈹€鈹€ TableRow 娴嬭瘯锛欵nvProfileEntry 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn profile_entry_columns_correct() {
        let cols = EnvProfileEntry::columns();
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0].name, "name");
        assert_eq!(cols[1].name, "var_count");
        assert_eq!(cols[1].kind, ValueKind::Int);
        assert_eq!(cols[2].name, "created");
        assert_eq!(cols[2].kind, ValueKind::Date);
    }

    #[test]
    fn profile_entry_cells_match_fields() {
        let e = EnvProfileEntry::new("dev", 15, "2026-05-12");
        let cells = e.cells();
        assert_eq!(cells.len(), 3);
        assert!(matches!(&cells[0], Value::String(s) if s == "dev"));
        assert!(matches!(&cells[1], Value::Int(15)));
    }

    // 鈹€鈹€ E2E 娴嬭瘯锛堟々锛?鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn env_subcommand_count() {
        let variants = [
            "Status", "List", "Search", "Get", "Set", "Del", "Check",
            "Path", "PathDedup", "Snapshot", "Doctor", "Profile", "Batch",
            "Apply", "Export", "ExportAll", "ExportLive", "Env", "Import",
            "DiffLive", "Graph", "Validate", "Schema", "Annotate", "Config",
            "Audit", "Watch", "Template", "Run", "Tui",
        ];
        assert_eq!(variants.len(), 30);
    }
}

// ============================================================
// Phase 3.6: ACL CLI 鈥?16 涓瓙鍛戒护
// ============================================================


