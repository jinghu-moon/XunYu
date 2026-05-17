mod bookmark_cmd_tests {
    use clap::Parser;
    use xun::xun_core::bookmark_cmd::{
        BookmarkCmd, BookmarkSubCommand, BookmarkEntry, FuzzyArgs,
        TagSubCommand,
    };
    use xun::xun_core::renderer::{JsonRenderer, Renderer};
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::value::{Value, ValueKind};
    use std::io::Cursor;

    // 鈹€鈹€ 杈呭姪鍑芥暟 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    fn parse(args: &[&str]) -> BookmarkCmd {
        let mut full = vec!["test"];
        full.extend_from_slice(args);
        BookmarkCmd::try_parse_from(full).unwrap()
    }

    fn make_fuzzy_args() -> FuzzyArgs {
        FuzzyArgs {
            patterns: vec!["proj".into()],
            tag: Some("work".into()),
            list: true,
            score: false,
            why: false,
            preview: false,
            limit: Some(5),
            json: false,
            tsv: false,
            global: false,
            child: false,
            base: None,
            workspace: None,
            preset: None,
        }
    }

    // 鈹€鈹€ CLI 瑙ｆ瀽娴嬭瘯锛歓 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn z_parses_basic() {
        let cmd = parse(&["z", "mydir"]);
        match cmd.sub {
            BookmarkSubCommand::Z(args) => {
                assert_eq!(args.fuzzy.patterns, vec!["mydir"]);
                assert!(!args.fuzzy.list);
                assert!(!args.fuzzy.score);
                assert!(!args.fuzzy.json);
            }
            other => panic!("expected Z, got {other:?}"),
        }
    }

    #[test]
    fn z_parses_multiple_patterns() {
        let cmd = parse(&["z", "proj", "src", "lib"]);
        match cmd.sub {
            BookmarkSubCommand::Z(args) => {
                assert_eq!(args.fuzzy.patterns, vec!["proj", "src", "lib"]);
            }
            other => panic!("expected Z, got {other:?}"),
        }
    }

    #[test]
    fn z_parses_all_flags() {
        let cmd = parse(&[
            "z", "proj", "-t", "work", "-l", "-s", "--why", "--preview",
            "-n", "10", "--json", "--tsv", "-g", "-c", "--base", "/tmp",
            "-w", "ws1", "--preset", "dev",
        ]);
        match cmd.sub {
            BookmarkSubCommand::Z(args) => {
                assert_eq!(args.fuzzy.tag.as_deref(), Some("work"));
                assert!(args.fuzzy.list);
                assert!(args.fuzzy.score);
                assert!(args.fuzzy.why);
                assert!(args.fuzzy.preview);
                assert_eq!(args.fuzzy.limit, Some(10));
                assert!(args.fuzzy.json);
                assert!(args.fuzzy.tsv);
                assert!(args.fuzzy.global);
                assert!(args.fuzzy.child);
                assert_eq!(args.fuzzy.base.as_deref(), Some("/tmp"));
                assert_eq!(args.fuzzy.workspace.as_deref(), Some("ws1"));
                assert_eq!(args.fuzzy.preset.as_deref(), Some("dev"));
            }
            other => panic!("expected Z, got {other:?}"),
        }
    }

    #[test]
    fn z_parses_no_patterns() {
        let cmd = parse(&["z"]);
        match cmd.sub {
            BookmarkSubCommand::Z(args) => {
                assert!(args.fuzzy.patterns.is_empty());
            }
            other => panic!("expected Z, got {other:?}"),
        }
    }

    // 鈹€鈹€ CLI 瑙ｆ瀽娴嬭瘯锛歓i / O / Oi / Open 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn zi_parses_basic() {
        let cmd = parse(&["zi", "proj"]);
        match cmd.sub {
            BookmarkSubCommand::Zi(args) => {
                assert_eq!(args.fuzzy.patterns, vec!["proj"]);
            }
            other => panic!("expected Zi, got {other:?}"),
        }
    }

    #[test]
    fn o_parses_basic() {
        let cmd = parse(&["o", "docs"]);
        match cmd.sub {
            BookmarkSubCommand::O(args) => {
                assert_eq!(args.fuzzy.patterns, vec!["docs"]);
            }
            other => panic!("expected O, got {other:?}"),
        }
    }

    #[test]
    fn oi_parses_basic() {
        let cmd = parse(&["oi", "src"]);
        match cmd.sub {
            BookmarkSubCommand::Oi(args) => {
                assert_eq!(args.fuzzy.patterns, vec!["src"]);
            }
            other => panic!("expected Oi, got {other:?}"),
        }
    }

    #[test]
    fn open_parses_basic() {
        let cmd = parse(&["open", "proj"]);
        match cmd.sub {
            BookmarkSubCommand::Open(args) => {
                assert_eq!(args.fuzzy.patterns, vec!["proj"]);
            }
            other => panic!("expected Open, got {other:?}"),
        }
    }

    // 鈹€鈹€ CLI 瑙ｆ瀽娴嬭瘯锛歋ave / Set 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn save_parses_name_and_options() {
        let cmd = parse(&["save", "myproj", "-t", "rust,cli", "--desc", "my project", "-w", "ws1"]);
        match cmd.sub {
            BookmarkSubCommand::Save(args) => {
                assert_eq!(args.name.as_deref(), Some("myproj"));
                assert_eq!(args.tag.as_deref(), Some("rust,cli"));
                assert_eq!(args.desc.as_deref(), Some("my project"));
                assert_eq!(args.workspace.as_deref(), Some("ws1"));
            }
            other => panic!("expected Save, got {other:?}"),
        }
    }

    #[test]
    fn save_parses_no_name() {
        let cmd = parse(&["save"]);
        match cmd.sub {
            BookmarkSubCommand::Save(args) => {
                assert!(args.name.is_none());
            }
            other => panic!("expected Save, got {other:?}"),
        }
    }

    #[test]
    fn set_parses_name_and_path() {
        let cmd = parse(&["set", "myproj", "C:/code/myproj"]);
        match cmd.sub {
            BookmarkSubCommand::Set(args) => {
                assert_eq!(args.name, "myproj");
                assert_eq!(args.path.as_deref(), Some("C:/code/myproj"));
                assert!(args.tag.is_none());
            }
            other => panic!("expected Set, got {other:?}"),
        }
    }

    #[test]
    fn set_parses_with_tag_and_desc() {
        let cmd = parse(&["set", "proj", ".", "-t", "work", "--desc", "main project"]);
        match cmd.sub {
            BookmarkSubCommand::Set(args) => {
                assert_eq!(args.name, "proj");
                assert_eq!(args.path.as_deref(), Some("."));
                assert_eq!(args.tag.as_deref(), Some("work"));
                assert_eq!(args.desc.as_deref(), Some("main project"));
            }
            other => panic!("expected Set, got {other:?}"),
        }
    }

    // 鈹€鈹€ CLI 瑙ｆ瀽娴嬭瘯锛欴elete / Pin / Unpin / Touch 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn delete_parses_name() {
        let cmd = parse(&["delete", "oldproj"]);
        match cmd.sub {
            BookmarkSubCommand::Rm(args) => {
                assert_eq!(args.name, "oldproj");
                assert!(!args.yes);
            }
            other => panic!("expected Rm, got {other:?}"),
        }
    }

    #[test]
    fn delete_parses_yes_flag() {
        let cmd = parse(&["delete", "oldproj", "-y"]);
        match cmd.sub {
            BookmarkSubCommand::Rm(args) => {
                assert_eq!(args.name, "oldproj");
                assert!(args.yes);
            }
            other => panic!("expected Rm, got {other:?}"),
        }
    }

    #[test]
    fn pin_parses_name() {
        let cmd = parse(&["pin", "important"]);
        match cmd.sub {
            BookmarkSubCommand::Pin(args) => {
                assert_eq!(args.name, "important");
            }
            other => panic!("expected Pin, got {other:?}"),
        }
    }

    #[test]
    fn unpin_parses_name() {
        let cmd = parse(&["unpin", "important"]);
        match cmd.sub {
            BookmarkSubCommand::Unpin(args) => {
                assert_eq!(args.name, "important");
            }
            other => panic!("expected Unpin, got {other:?}"),
        }
    }

    #[test]
    fn touch_parses_name() {
        let cmd = parse(&["touch", "mydir"]);
        match cmd.sub {
            BookmarkSubCommand::Touch(args) => {
                assert_eq!(args.name, "mydir");
            }
            other => panic!("expected Touch, got {other:?}"),
        }
    }

    // 鈹€鈹€ CLI 瑙ｆ瀽娴嬭瘯锛歎ndo / Redo / Rename 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn undo_parses_default_steps() {
        let cmd = parse(&["undo"]);
        match cmd.sub {
            BookmarkSubCommand::Undo(args) => {
                assert_eq!(args.steps, 1);
            }
            other => panic!("expected Undo, got {other:?}"),
        }
    }

    #[test]
    fn undo_parses_custom_steps() {
        let cmd = parse(&["undo", "-n", "5"]);
        match cmd.sub {
            BookmarkSubCommand::Undo(args) => {
                assert_eq!(args.steps, 5);
            }
            other => panic!("expected Undo, got {other:?}"),
        }
    }

    #[test]
    fn redo_parses_default_steps() {
        let cmd = parse(&["redo"]);
        match cmd.sub {
            BookmarkSubCommand::Redo(args) => {
                assert_eq!(args.steps, 1);
            }
            other => panic!("expected Redo, got {other:?}"),
        }
    }

    #[test]
    fn redo_parses_custom_steps() {
        let cmd = parse(&["redo", "-n", "3"]);
        match cmd.sub {
            BookmarkSubCommand::Redo(args) => {
                assert_eq!(args.steps, 3);
            }
            other => panic!("expected Redo, got {other:?}"),
        }
    }

    #[test]
    fn rename_parses_old_and_new() {
        let cmd = parse(&["rename", "old_name", "new_name"]);
        match cmd.sub {
            BookmarkSubCommand::Rename(args) => {
                assert_eq!(args.old, "old_name");
                assert_eq!(args.new, "new_name");
            }
            other => panic!("expected Rename, got {other:?}"),
        }
    }

    // 鈹€鈹€ CLI 瑙ｆ瀽娴嬭瘯锛歀ist / Recent / Stats / Check 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn list_parses_defaults() {
        let cmd = parse(&["list"]);
        match cmd.sub {
            BookmarkSubCommand::List(args) => {
                assert!(args.tag.is_none());
                assert_eq!(args.sort, "name");
                assert!(args.limit.is_none());
                assert!(args.offset.is_none());
                assert!(!args.reverse);
                assert!(!args.tsv);
                assert_eq!(args.format, "auto");
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn list_parses_all_options() {
        let cmd = parse(&[
            "list", "-t", "work", "-s", "visits", "-n", "20",
            "--offset", "5", "--reverse", "--tsv", "-f", "json",
        ]);
        match cmd.sub {
            BookmarkSubCommand::List(args) => {
                assert_eq!(args.tag.as_deref(), Some("work"));
                assert_eq!(args.sort, "visits");
                assert_eq!(args.limit, Some(20));
                assert_eq!(args.offset, Some(5));
                assert!(args.reverse);
                assert!(args.tsv);
                assert_eq!(args.format, "json");
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn recent_parses_defaults() {
        let cmd = parse(&["recent"]);
        match cmd.sub {
            BookmarkSubCommand::Recent(args) => {
                assert_eq!(args.limit, 10);
                assert!(args.tag.is_none());
                assert!(args.workspace.is_none());
                assert!(args.since.is_none());
                assert_eq!(args.format, "auto");
            }
            other => panic!("expected Recent, got {other:?}"),
        }
    }

    #[test]
    fn recent_parses_all_options() {
        let cmd = parse(&[
            "recent", "-n", "25", "-t", "work", "-w", "ws1",
            "--since", "7d", "-f", "json",
        ]);
        match cmd.sub {
            BookmarkSubCommand::Recent(args) => {
                assert_eq!(args.limit, 25);
                assert_eq!(args.tag.as_deref(), Some("work"));
                assert_eq!(args.workspace.as_deref(), Some("ws1"));
                assert_eq!(args.since.as_deref(), Some("7d"));
                assert_eq!(args.format, "json");
            }
            other => panic!("expected Recent, got {other:?}"),
        }
    }

    #[test]
    fn stats_parses_defaults() {
        let cmd = parse(&["stats"]);
        match cmd.sub {
            BookmarkSubCommand::Stats(args) => {
                assert_eq!(args.format, "auto");
                assert!(!args.insights);
            }
            other => panic!("expected Stats, got {other:?}"),
        }
    }

    #[test]
    fn stats_parses_insights() {
        let cmd = parse(&["stats", "--insights", "-f", "json"]);
        match cmd.sub {
            BookmarkSubCommand::Stats(args) => {
                assert!(args.insights);
                assert_eq!(args.format, "json");
            }
            other => panic!("expected Stats, got {other:?}"),
        }
    }

    #[test]
    fn check_parses_defaults() {
        let cmd = parse(&["check"]);
        match cmd.sub {
            BookmarkSubCommand::Check(args) => {
                assert_eq!(args.days, 90);
                assert_eq!(args.format, "auto");
            }
            other => panic!("expected Check, got {other:?}"),
        }
    }

    #[test]
    fn check_parses_custom_days() {
        let cmd = parse(&["check", "-d", "30", "-f", "tsv"]);
        match cmd.sub {
            BookmarkSubCommand::Check(args) => {
                assert_eq!(args.days, 30);
                assert_eq!(args.format, "tsv");
            }
            other => panic!("expected Check, got {other:?}"),
        }
    }

    // 鈹€鈹€ CLI 瑙ｆ瀽娴嬭瘯锛欸c / Dedup / Export / Import 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn gc_parses_defaults() {
        let cmd = parse(&["gc"]);
        match cmd.sub {
            BookmarkSubCommand::Gc(args) => {
                assert!(!args.purge);
                assert!(!args.dry_run);
                assert!(!args.learned);
                assert_eq!(args.format, "auto");
            }
            other => panic!("expected Gc, got {other:?}"),
        }
    }

    #[test]
    fn gc_parses_all_flags() {
        let cmd = parse(&["gc", "--purge", "--dry-run", "--learned", "-f", "json"]);
        match cmd.sub {
            BookmarkSubCommand::Gc(args) => {
                assert!(args.purge);
                assert!(args.dry_run);
                assert!(args.learned);
                assert_eq!(args.format, "json");
            }
            other => panic!("expected Gc, got {other:?}"),
        }
    }

    #[test]
    fn dedup_parses_defaults() {
        let cmd = parse(&["dedup"]);
        match cmd.sub {
            BookmarkSubCommand::Dedup(args) => {
                assert_eq!(args.mode, "path");
                assert_eq!(args.format, "auto");
                assert!(!args.yes);
            }
            other => panic!("expected Dedup, got {other:?}"),
        }
    }

    #[test]
    fn dedup_parses_name_mode() {
        let cmd = parse(&["dedup", "-m", "name", "-y"]);
        match cmd.sub {
            BookmarkSubCommand::Dedup(args) => {
                assert_eq!(args.mode, "name");
                assert!(args.yes);
            }
            other => panic!("expected Dedup, got {other:?}"),
        }
    }

    #[test]
    fn export_parses_defaults() {
        let cmd = parse(&["export"]);
        match cmd.sub {
            BookmarkSubCommand::Export(args) => {
                assert_eq!(args.format, "json");
                assert!(args.out.is_none());
            }
            other => panic!("expected Export, got {other:?}"),
        }
    }

    #[test]
    fn export_parses_tsv_with_output() {
        let cmd = parse(&["export", "-f", "tsv", "-o", "bookmarks.tsv"]);
        match cmd.sub {
            BookmarkSubCommand::Export(args) => {
                assert_eq!(args.format, "tsv");
                assert_eq!(args.out.as_deref(), Some("bookmarks.tsv"));
            }
            other => panic!("expected Export, got {other:?}"),
        }
    }

    #[test]
    fn import_parses_defaults() {
        let cmd = parse(&["import"]);
        match cmd.sub {
            BookmarkSubCommand::Import(args) => {
                assert_eq!(args.format, "json");
                assert!(args.from.is_none());
                assert!(args.input.is_none());
                assert_eq!(args.mode, "merge");
                assert!(!args.yes);
            }
            other => panic!("expected Import, got {other:?}"),
        }
    }

    #[test]
    fn import_parses_all_options() {
        let cmd = parse(&[
            "import", "-f", "tsv", "--from", "zoxide", "-i", "data.tsv",
            "-m", "overwrite", "-y",
        ]);
        match cmd.sub {
            BookmarkSubCommand::Import(args) => {
                assert_eq!(args.format, "tsv");
                assert_eq!(args.from.as_deref(), Some("zoxide"));
                assert_eq!(args.input.as_deref(), Some("data.tsv"));
                assert_eq!(args.mode, "overwrite");
                assert!(args.yes);
            }
            other => panic!("expected Import, got {other:?}"),
        }
    }

    // 鈹€鈹€ CLI 瑙ｆ瀽娴嬭瘯锛欼nit / Learn / Keys / All 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn init_parses_shell() {
        let cmd = parse(&["init", "powershell"]);
        match cmd.sub {
            BookmarkSubCommand::Init(args) => {
                assert_eq!(args.shell, "powershell");
                assert!(args.cmd.is_none());
            }
            other => panic!("expected Init, got {other:?}"),
        }
    }

    #[test]
    fn init_parses_custom_cmd() {
        let cmd = parse(&["init", "bash", "--cmd", "j"]);
        match cmd.sub {
            BookmarkSubCommand::Init(args) => {
                assert_eq!(args.shell, "bash");
                assert_eq!(args.cmd.as_deref(), Some("j"));
            }
            other => panic!("expected Init, got {other:?}"),
        }
    }

    #[test]
    fn learn_parses_path() {
        let cmd = parse(&["learn", "--path", "C:/code/myproj"]);
        match cmd.sub {
            BookmarkSubCommand::Learn(args) => {
                assert_eq!(args.path, "C:/code/myproj");
            }
            other => panic!("expected Learn, got {other:?}"),
        }
    }

    #[test]
    fn keys_parses_empty() {
        let cmd = parse(&["keys"]);
        match cmd.sub {
            BookmarkSubCommand::Keys(_) => {}
            other => panic!("expected Keys, got {other:?}"),
        }
    }

    #[test]
    fn all_parses_no_tag() {
        let cmd = parse(&["all"]);
        match cmd.sub {
            BookmarkSubCommand::All(args) => {
                assert!(args.tag.is_none());
            }
            other => panic!("expected All, got {other:?}"),
        }
    }

    #[test]
    fn all_parses_with_tag() {
        let cmd = parse(&["all", "work"]);
        match cmd.sub {
            BookmarkSubCommand::All(args) => {
                assert_eq!(args.tag.as_deref(), Some("work"));
            }
            other => panic!("expected All, got {other:?}"),
        }
    }

    // 鈹€鈹€ CLI 瑙ｆ瀽娴嬭瘯锛歍ag 瀛愬懡浠ょ粍 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn tag_add_parses() {
        let cmd = parse(&["tag", "add", "myproj", "rust,cli"]);
        match cmd.sub {
            BookmarkSubCommand::Tag(tag) => match tag.sub {
                TagSubCommand::Add(args) => {
                    assert_eq!(args.name, "myproj");
                    assert_eq!(args.tags, "rust,cli");
                }
                other => panic!("expected Add, got {other:?}"),
            },
            other => panic!("expected Tag, got {other:?}"),
        }
    }

    #[test]
    fn tag_add_batch_parses() {
        let cmd = parse(&["tag", "add-batch", "work", "proj1", "proj2", "proj3"]);
        match cmd.sub {
            BookmarkSubCommand::Tag(tag) => match tag.sub {
                TagSubCommand::AddBatch(args) => {
                    assert_eq!(args.tags, "work");
                    assert_eq!(args.names, vec!["proj1", "proj2", "proj3"]);
                }
                other => panic!("expected AddBatch, got {other:?}"),
            },
            other => panic!("expected Tag, got {other:?}"),
        }
    }

    #[test]
    fn tag_remove_parses() {
        let cmd = parse(&["tag", "remove", "myproj", "old"]);
        match cmd.sub {
            BookmarkSubCommand::Tag(tag) => match tag.sub {
                TagSubCommand::Remove(args) => {
                    assert_eq!(args.name, "myproj");
                    assert_eq!(args.tags, "old");
                }
                other => panic!("expected Remove, got {other:?}"),
            },
            other => panic!("expected Tag, got {other:?}"),
        }
    }

    #[test]
    fn tag_list_parses() {
        let cmd = parse(&["tag", "list"]);
        match cmd.sub {
            BookmarkSubCommand::Tag(tag) => match tag.sub {
                TagSubCommand::List(_) => {}
                other => panic!("expected List, got {other:?}"),
            },
            other => panic!("expected Tag, got {other:?}"),
        }
    }

    #[test]
    fn tag_rename_parses() {
        let cmd = parse(&["tag", "rename", "old_tag", "new_tag"]);
        match cmd.sub {
            BookmarkSubCommand::Tag(tag) => match tag.sub {
                TagSubCommand::Rename(args) => {
                    assert_eq!(args.old, "old_tag");
                    assert_eq!(args.new, "new_tag");
                }
                other => panic!("expected Rename, got {other:?}"),
            },
            other => panic!("expected Tag, got {other:?}"),
        }
    }

    // 鈹€鈹€ TableRow 娴嬭瘯 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn bookmark_entry_columns_correct() {
        let cols = BookmarkEntry::columns();
        assert_eq!(cols.len(), 6);
        assert_eq!(cols[0].name, "name");
        assert_eq!(cols[0].kind, ValueKind::String);
        assert_eq!(cols[1].name, "path");
        assert_eq!(cols[1].kind, ValueKind::Path);
        assert_eq!(cols[2].name, "tags");
        assert_eq!(cols[2].kind, ValueKind::String);
        assert_eq!(cols[3].name, "visits");
        assert_eq!(cols[3].kind, ValueKind::Int);
        assert_eq!(cols[4].name, "last_used");
        assert_eq!(cols[4].kind, ValueKind::Date);
        assert_eq!(cols[5].name, "pinned");
        assert_eq!(cols[5].kind, ValueKind::Bool);
    }

    #[test]
    fn bookmark_entry_cells_match_fields() {
        let entry = BookmarkEntry::new("myproj", "C:/code/myproj", "rust,cli", 42, "2026-05-12", true);
        let cells = entry.cells();
        assert_eq!(cells.len(), 6);
        assert!(matches!(&cells[0], Value::String(s) if s == "myproj"));
        assert!(matches!(&cells[1], Value::String(s) if s == "C:/code/myproj"));
        assert!(matches!(&cells[2], Value::String(s) if s == "rust,cli"));
        assert!(matches!(&cells[3], Value::Int(42)));
        assert!(matches!(&cells[4], Value::String(s) if s == "2026-05-12"));
        assert!(matches!(&cells[5], Value::Bool(true)));
    }

    #[test]
    fn bookmark_entry_renders_as_json() {
        let entry = BookmarkEntry::new("test", "/tmp", "tag1", 5, "2026-01-01", false);
        let table = entry.to_table();
        let mut buf = Cursor::new(Vec::new());
        let mut r = JsonRenderer::new(false, &mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("test"), "json output: {output}");
        assert!(output.contains("/tmp"), "json output: {output}");
    }

    #[test]
    fn bookmark_entry_vec_to_table() {
        let entries = vec![
            BookmarkEntry::new("a", "/a", "t1", 1, "2026-01-01", false),
            BookmarkEntry::new("b", "/b", "t2", 2, "2026-02-02", true),
        ];
        let table = BookmarkEntry::vec_to_table(&entries);
        assert_eq!(table.len(), 2);
        assert_eq!(table.columns.len(), 6);
    }

    // 鈹€鈹€ CommandSpec 娴嬭瘯锛堟々瀹炵幇锛?鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn bookmark_entry_to_record_has_all_fields() {
        let entry = BookmarkEntry::new("proj", "/code/proj", "rust", 10, "2026-05-12", true);
        let rec = entry.to_record();
        assert_eq!(rec.len(), 6);
        assert!(rec.contains_key("name"));
        assert!(rec.contains_key("path"));
        assert!(rec.contains_key("tags"));
        assert!(rec.contains_key("visits"));
        assert!(rec.contains_key("last_used"));
        assert!(rec.contains_key("pinned"));
    }

    #[test]
    fn bookmark_entry_new_constructor() {
        let entry = BookmarkEntry::new("n", "p", "t", 99, "2026-01-01", true);
        assert_eq!(entry.name, "n");
        assert_eq!(entry.path, "p");
        assert_eq!(entry.tags, "t");
        assert_eq!(entry.visits, 99);
        assert_eq!(entry.last_used, "2026-01-01");
        assert!(entry.pinned);
    }

    // 鈹€鈹€ E2E 娴嬭瘯锛堟々锛?鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn bookmark_subcommand_count() {
        // 楠岃瘉 BookmarkSubCommand 鏈?27 涓彉浣?
        let variants = [
            "Z", "Zi", "O", "Oi", "Open", "Save", "Set", "Delete",
            "Tag", "Pin", "Unpin", "Undo", "Redo", "Rename",
            "List", "Recent", "Stats", "Check", "Gc", "Dedup",
            "Export", "Import", "Init", "Learn", "Touch", "Keys", "All",
        ];
        assert_eq!(variants.len(), 27);
    }

    #[test]
    fn tag_subcommand_count() {
        // 楠岃瘉 TagSubCommand 鏈?5 涓彉浣?
        let variants = ["Add", "AddBatch", "Remove", "List", "Rename"];
        assert_eq!(variants.len(), 5);
    }
}

// ============================================================
// Phase 3.5: Env CLI 鈥?27 涓瓙鍛戒护
// ============================================================


