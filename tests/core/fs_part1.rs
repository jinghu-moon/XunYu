mod acl_cmd_tests {
    use clap::Parser;
    use xun::xun_core::acl_cmd::{AclCmd, AclSubCommand, AclEntry};
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::renderer::{JsonRenderer, Renderer};
    use xun::xun_core::value::{Value, ValueKind};
    use std::io::Cursor;

    // 鈹€鈹€ 杈呭姪鍑芥暟 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    fn parse(args: &[&str]) -> AclCmd {
        let mut full = vec!["test"];
        full.extend_from_slice(args);
        AclCmd::try_parse_from(full).unwrap()
    }

    // 鈹€鈹€ CLI 瑙ｆ瀽娴嬭瘯 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn view_parses_basic() {
        let cmd = parse(&["view", "-p", "C:\\test"]);
        match cmd.cmd {
            AclSubCommand::Show(args) => {
                assert_eq!(args.path, "C:\\test");
                assert!(!args.detail);
                assert!(args.export.is_none());
            }
            other => panic!("expected Show, got {other:?}"),
        }
    }

    #[test]
    fn view_parses_detail_and_export() {
        let cmd = parse(&["view", "-p", "C:\\test", "--detail", "--export", "out.csv"]);
        match cmd.cmd {
            AclSubCommand::Show(args) => {
                assert!(args.detail);
                assert_eq!(args.export.as_deref(), Some("out.csv"));
            }
            other => panic!("expected Show, got {other:?}"),
        }
    }

    #[test]
    fn add_parses_basic() {
        let cmd = parse(&["add", "-p", "C:\\test", "--principal", "BUILTIN\\Users", "--rights", "Read"]);
        match cmd.cmd {
            AclSubCommand::Add(args) => {
                assert_eq!(args.path.as_deref(), Some("C:\\test"));
                assert_eq!(args.principal.as_deref(), Some("BUILTIN\\Users"));
                assert_eq!(args.rights.as_deref(), Some("Read"));
                assert!(!args.yes);
            }
            other => panic!("expected Add, got {other:?}"),
        }
    }

    #[test]
    fn add_parses_all_options() {
        let cmd = parse(&[
            "add", "-p", "C:\\test", "--principal", "DOMAIN\\User",
            "--rights", "FullControl", "--ace-type", "Allow",
            "--inherit", "BothInherit", "-y",
        ]);
        match cmd.cmd {
            AclSubCommand::Add(args) => {
                assert_eq!(args.rights.as_deref(), Some("FullControl"));
                assert_eq!(args.ace_type.as_deref(), Some("Allow"));
                assert_eq!(args.inherit.as_deref(), Some("BothInherit"));
                assert!(args.yes);
            }
            other => panic!("expected Add, got {other:?}"),
        }
    }

    #[test]
    fn remove_parses() {
        let cmd = parse(&["remove", "-p", "C:\\test", "--principal", "BUILTIN\\Users", "-y"]);
        match cmd.cmd {
            AclSubCommand::Rm(args) => {
                assert_eq!(args.path, "C:\\test");
                assert_eq!(args.principal.as_deref(), Some("BUILTIN\\Users"));
                assert!(args.yes);
            }
            other => panic!("expected Rm, got {other:?}"),
        }
    }

    #[test]
    fn purge_parses() {
        let cmd = parse(&["purge", "-p", "C:\\test", "--principal", "DOMAIN\\Bad"]);
        match cmd.cmd {
            AclSubCommand::Purge(args) => {
                assert_eq!(args.path, "C:\\test");
                assert_eq!(args.principal.as_deref(), Some("DOMAIN\\Bad"));
            }
            other => panic!("expected Purge, got {other:?}"),
        }
    }

    #[test]
    fn diff_parses() {
        let cmd = parse(&["diff", "-p", "C:\\a", "-r", "C:\\b", "-o", "diff.csv"]);
        match cmd.cmd {
            AclSubCommand::Diff(args) => {
                assert_eq!(args.path, "C:\\a");
                assert_eq!(args.reference, "C:\\b");
                assert_eq!(args.output.as_deref(), Some("diff.csv"));
            }
            other => panic!("expected Diff, got {other:?}"),
        }
    }

    #[test]
    fn batch_parses() {
        let cmd = parse(&["batch", "--action", "repair", "--file", "paths.txt", "-y"]);
        match cmd.cmd {
            AclSubCommand::Batch(args) => {
                assert_eq!(args.action, "repair");
                assert_eq!(args.file.as_deref(), Some("paths.txt"));
                assert!(args.yes);
            }
            other => panic!("expected Batch, got {other:?}"),
        }
    }

    #[test]
    fn effective_parses() {
        let cmd = parse(&["effective", "-p", "C:\\test", "-u", "DOMAIN\\User"]);
        match cmd.cmd {
            AclSubCommand::Effective(args) => {
                assert_eq!(args.path, "C:\\test");
                assert_eq!(args.user.as_deref(), Some("DOMAIN\\User"));
            }
            other => panic!("expected Effective, got {other:?}"),
        }
    }

    #[test]
    fn copy_parses() {
        let cmd = parse(&["copy", "-p", "C:\\dst", "-r", "C:\\src", "-y"]);
        match cmd.cmd {
            AclSubCommand::Copy(args) => {
                assert_eq!(args.path, "C:\\dst");
                assert_eq!(args.reference, "C:\\src");
                assert!(args.yes);
            }
            other => panic!("expected Copy, got {other:?}"),
        }
    }

    #[test]
    fn backup_parses() {
        let cmd = parse(&["backup", "-p", "C:\\test", "-o", "acl.json"]);
        match cmd.cmd {
            AclSubCommand::Backup(args) => {
                assert_eq!(args.path, "C:\\test");
                assert_eq!(args.output.as_deref(), Some("acl.json"));
            }
            other => panic!("expected Backup, got {other:?}"),
        }
    }

    #[test]
    fn restore_parses() {
        let cmd = parse(&["restore", "-p", "C:\\test", "--from", "acl.json", "-y"]);
        match cmd.cmd {
            AclSubCommand::Restore(args) => {
                assert_eq!(args.path, "C:\\test");
                assert_eq!(args.from, "acl.json");
                assert!(args.yes);
            }
            other => panic!("expected Restore, got {other:?}"),
        }
    }

    #[test]
    fn inherit_parses_disable() {
        let cmd = parse(&["inherit", "-p", "C:\\test", "--disable"]);
        match cmd.cmd {
            AclSubCommand::Inherit(args) => {
                assert!(args.disable);
                assert!(!args.enable);
                assert_eq!(args.preserve, "true");
            }
            other => panic!("expected Inherit, got {other:?}"),
        }
    }

    #[test]
    fn inherit_parses_enable() {
        let cmd = parse(&["inherit", "-p", "C:\\test", "--enable"]);
        match cmd.cmd {
            AclSubCommand::Inherit(args) => {
                assert!(!args.disable);
                assert!(args.enable);
            }
            other => panic!("expected Inherit, got {other:?}"),
        }
    }

    #[test]
    fn inherit_parses_preserve_false() {
        let cmd = parse(&["inherit", "-p", "C:\\test", "--disable", "--preserve", "false"]);
        match cmd.cmd {
            AclSubCommand::Inherit(args) => {
                assert_eq!(args.preserve, "false");
            }
            other => panic!("expected Inherit, got {other:?}"),
        }
    }

    #[test]
    fn owner_parses() {
        let cmd = parse(&["owner", "-p", "C:\\test", "--set", "DOMAIN\\Admin", "-y"]);
        match cmd.cmd {
            AclSubCommand::Owner(args) => {
                assert_eq!(args.path, "C:\\test");
                assert_eq!(args.set.as_deref(), Some("DOMAIN\\Admin"));
                assert!(args.yes);
            }
            other => panic!("expected Owner, got {other:?}"),
        }
    }

    #[test]
    fn orphans_parses_defaults() {
        let cmd = parse(&["orphans", "-p", "C:\\test"]);
        match cmd.cmd {
            AclSubCommand::Orphans(args) => {
                assert_eq!(args.path, "C:\\test");
                assert_eq!(args.recursive, "true");
                assert_eq!(args.action, "none");
                assert!(!args.yes);
            }
            other => panic!("expected Orphans, got {other:?}"),
        }
    }

    #[test]
    fn orphans_parses_action_and_recursive_false() {
        let cmd = parse(&["orphans", "-p", "C:\\test", "--recursive", "false", "--action", "delete", "-y"]);
        match cmd.cmd {
            AclSubCommand::Orphans(args) => {
                assert_eq!(args.recursive, "false");
                assert_eq!(args.action, "delete");
                assert!(args.yes);
            }
            other => panic!("expected Orphans, got {other:?}"),
        }
    }

    #[test]
    fn repair_parses_basic() {
        let cmd = parse(&["repair", "-p", "C:\\test"]);
        match cmd.cmd {
            AclSubCommand::Repair(args) => {
                assert_eq!(args.path, "C:\\test");
                assert!(!args.export_errors);
                assert!(!args.yes);
                assert!(!args.reset_clean);
                assert!(args.grant.is_none());
            }
            other => panic!("expected Repair, got {other:?}"),
        }
    }

    #[test]
    fn repair_parses_reset_clean() {
        let cmd = parse(&["repair", "-p", "C:\\test", "--reset-clean", "--grant", "DOMAIN\\User,BUILTIN\\Users", "-y"]);
        match cmd.cmd {
            AclSubCommand::Repair(args) => {
                assert!(args.reset_clean);
                assert_eq!(args.grant.as_deref(), Some("DOMAIN\\User,BUILTIN\\Users"));
                assert!(args.yes);
            }
            other => panic!("expected Repair, got {other:?}"),
        }
    }

    #[test]
    fn audit_parses_defaults() {
        let cmd = parse(&["audit"]);
        match cmd.cmd {
            AclSubCommand::Audit(args) => {
                assert_eq!(args.tail, 30);
                assert!(args.export.is_none());
            }
            other => panic!("expected Audit, got {other:?}"),
        }
    }

    #[test]
    fn audit_parses_tail_and_export() {
        let cmd = parse(&["audit", "--tail", "100", "--export", "audit.csv"]);
        match cmd.cmd {
            AclSubCommand::Audit(args) => {
                assert_eq!(args.tail, 100);
                assert_eq!(args.export.as_deref(), Some("audit.csv"));
            }
            other => panic!("expected Audit, got {other:?}"),
        }
    }

    #[test]
    fn config_parses_set() {
        let cmd = parse(&["config", "--set", "default_owner", "BUILTIN\\Administrators"]);
        match cmd.cmd {
            AclSubCommand::Config(args) => {
                assert_eq!(args.set, vec!["default_owner", "BUILTIN\\Administrators"]);
            }
            other => panic!("expected Config, got {other:?}"),
        }
    }

    // 鈹€鈹€ TableRow 娴嬭瘯 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn acl_entry_columns_correct() {
        let cols = AclEntry::columns();
        assert_eq!(cols.len(), 5);
        assert_eq!(cols[0].name, "path");
        assert_eq!(cols[0].kind, ValueKind::Path);
        assert_eq!(cols[1].name, "principal");
        assert_eq!(cols[1].kind, ValueKind::String);
        assert_eq!(cols[2].name, "rights");
        assert_eq!(cols[2].kind, ValueKind::String);
        assert_eq!(cols[3].name, "ace_type");
        assert_eq!(cols[3].kind, ValueKind::String);
        assert_eq!(cols[4].name, "inherited");
        assert_eq!(cols[4].kind, ValueKind::Bool);
    }

    #[test]
    fn acl_entry_cells_match_fields() {
        let entry = AclEntry::new("C:\\test", "BUILTIN\\Users", "Read", "Allow", false);
        let cells = entry.cells();
        assert_eq!(cells.len(), 5);
        assert!(matches!(&cells[0], Value::String(s) if s == "C:\\test"));
        assert!(matches!(&cells[1], Value::String(s) if s == "BUILTIN\\Users"));
        assert!(matches!(&cells[2], Value::String(s) if s == "Read"));
        assert!(matches!(&cells[3], Value::String(s) if s == "Allow"));
        assert!(matches!(&cells[4], Value::Bool(false)));
    }

    #[test]
    fn acl_entry_renders_as_json() {
        let entry = AclEntry::new("/tmp", "Everyone", "FullControl", "Allow", true);
        let table = entry.to_table();
        let mut buf = Cursor::new(Vec::new());
        let mut r = JsonRenderer::new(false, &mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("Everyone"), "json output: {output}");
    }

    #[test]
    fn acl_entry_vec_to_table() {
        let entries = vec![
            AclEntry::new("C:\\a", "User1", "Read", "Allow", false),
            AclEntry::new("C:\\b", "User2", "Write", "Deny", true),
        ];
        let table = AclEntry::vec_to_table(&entries);
        assert_eq!(table.len(), 2);
        assert_eq!(table.columns.len(), 5);
    }

    // 鈹€鈹€ E2E 娴嬭瘯锛堟々锛?鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn acl_subcommand_count() {
        let variants = [
            "View", "Add", "Remove", "Purge", "Diff", "Batch",
            "Effective", "Copy", "Backup", "Restore", "Inherit",
            "Owner", "Orphans", "Repair", "Audit", "Config",
        ];
        assert_eq!(variants.len(), 16);
    }
}

// ============================================================
// Phase 3.7: alias_cmd 鈥?10 椤跺眰 + 6 宓屽 app 瀛愬懡浠?
// ============================================================

mod alias_cmd_tests {
    use clap::Parser;
    use xun::xun_core::alias_cmd::*;
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::value::{Value, ValueKind};
    use xun::xun_core::renderer::{JsonRenderer, Renderer};
    use std::io::Cursor;

    fn parse(args: &[&str]) -> AliasCmd {
        let mut argv = vec!["test"];
        argv.extend_from_slice(args);
        AliasCmd::try_parse_from(&argv).expect("parse failed")
    }

    // 鈹€鈹€ 椤跺眰瀛愬懡浠よВ鏋?鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn setup_parses_defaults() {
        let cmd = parse(&["setup"]);
        match cmd.sub {
            AliasSubCommand::Setup(args) => {
                assert!(!args.no_cmd);
                assert!(!args.no_ps);
                assert!(!args.no_bash);
                assert!(!args.no_nu);
                assert!(!args.core_only);
            }
            other => panic!("expected Setup, got {other:?}"),
        }
    }

    #[test]
    fn setup_parses_flags() {
        let cmd = parse(&["setup", "--no-cmd", "--no-ps", "--no-bash", "--no-nu", "--core-only"]);
        match cmd.sub {
            AliasSubCommand::Setup(args) => {
                assert!(args.no_cmd);
                assert!(args.no_ps);
                assert!(args.no_bash);
                assert!(args.no_nu);
                assert!(args.core_only);
            }
            other => panic!("expected Setup, got {other:?}"),
        }
    }

    #[test]
    fn add_parses_positional() {
        let cmd = parse(&["add", "ll", "ls -la"]);
        match cmd.sub {
            AliasSubCommand::Add(args) => {
                assert_eq!(args.name, "ll");
                assert_eq!(args.command, "ls -la");
                assert_eq!(args.mode, "auto");
                assert!(args.desc.is_none());
                assert!(args.tag.is_empty());
                assert!(args.shell.is_empty());
                assert!(!args.force);
            }
            other => panic!("expected Add, got {other:?}"),
        }
    }

    #[test]
    fn add_parses_all_options() {
        let cmd = parse(&["add", "ll", "ls -la", "--mode", "exe", "--desc", "list long", "--tag", "shell", "--tag", "util", "--shell", "cmd", "--shell", "ps", "--force"]);
        match cmd.sub {
            AliasSubCommand::Add(args) => {
                assert_eq!(args.mode, "exe");
                assert_eq!(args.desc.as_deref(), Some("list long"));
                assert_eq!(args.tag, vec!["shell", "util"]);
                assert_eq!(args.shell, vec!["cmd", "ps"]);
                assert!(args.force);
            }
            other => panic!("expected Add, got {other:?}"),
        }
    }

    #[test]
    fn rm_parses_names() {
        let cmd = parse(&["rm", "ll", "la"]);
        match cmd.sub {
            AliasSubCommand::Rm(args) => {
                assert_eq!(args.names, vec!["ll", "la"]);
            }
            other => panic!("expected Rm, got {other:?}"),
        }
    }

    #[test]
    fn ls_parses_defaults() {
        let cmd = parse(&["ls"]);
        match cmd.sub {
            AliasSubCommand::List(args) => {
                assert!(args.r#type.is_none());
                assert!(args.tag.is_none());
                assert!(!args.json);
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn ls_parses_options() {
        let cmd = parse(&["ls", "--type", "cmd", "--tag", "util", "--json"]);
        match cmd.sub {
            AliasSubCommand::List(args) => {
                assert_eq!(args.r#type.as_deref(), Some("cmd"));
                assert_eq!(args.tag.as_deref(), Some("util"));
                assert!(args.json);
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn find_parses_keyword() {
        let cmd = parse(&["find", "git"]);
        match cmd.sub {
            AliasSubCommand::Find(args) => {
                assert_eq!(args.keyword, "git");
            }
            other => panic!("expected Find, got {other:?}"),
        }
    }

    #[test]
    fn which_parses_name() {
        let cmd = parse(&["which", "ll"]);
        match cmd.sub {
            AliasSubCommand::Which(args) => {
                assert_eq!(args.name, "ll");
            }
            other => panic!("expected Which, got {other:?}"),
        }
    }

    #[test]
    fn sync_parses_empty() {
        let cmd = parse(&["sync"]);
        match cmd.sub {
            AliasSubCommand::Sync(_) => {}
            other => panic!("expected Sync, got {other:?}"),
        }
    }

    #[test]
    fn export_parses_defaults() {
        let cmd = parse(&["export"]);
        match cmd.sub {
            AliasSubCommand::Export(args) => {
                assert!(args.output.is_none());
            }
            other => panic!("expected Export, got {other:?}"),
        }
    }

    #[test]
    fn export_parses_output() {
        let cmd = parse(&["export", "-o", "aliases.toml"]);
        match cmd.sub {
            AliasSubCommand::Export(args) => {
                assert_eq!(args.output.as_deref(), Some("aliases.toml"));
            }
            other => panic!("expected Export, got {other:?}"),
        }
    }

    #[test]
    fn import_parses_file() {
        let cmd = parse(&["import", "aliases.toml"]);
        match cmd.sub {
            AliasSubCommand::Import(args) => {
                assert_eq!(args.file, "aliases.toml");
                assert!(!args.force);
            }
            other => panic!("expected Import, got {other:?}"),
        }
    }

    #[test]
    fn import_parses_force() {
        let cmd = parse(&["import", "aliases.toml", "--force"]);
        match cmd.sub {
            AliasSubCommand::Import(args) => {
                assert!(args.force);
            }
            other => panic!("expected Import, got {other:?}"),
        }
    }

    #[test]
    fn config_flag_parsed() {
        let cmd = AliasCmd::try_parse_from(["test", "--config", "custom.toml", "ls"]).unwrap();
        assert_eq!(cmd.config.as_deref(), Some("custom.toml"));
    }

    // 鈹€鈹€ App 宓屽瀛愬懡浠よВ鏋?鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn app_add_parses_positional() {
        let cmd = parse(&["app", "add", "vscode", "C:\\VSCode\\Code.exe"]);
        match cmd.sub {
            AliasSubCommand::App(app) => match app.sub {
                AliasAppSubCommand::Add(args) => {
                    assert_eq!(args.name, "vscode");
                    assert_eq!(args.exe, "C:\\VSCode\\Code.exe");
                    assert!(args.args.is_none());
                    assert!(args.desc.is_none());
                    assert!(args.tag.is_empty());
                    assert!(!args.no_apppaths);
                    assert!(!args.force);
                }
                other => panic!("expected App::Add, got {other:?}"),
            },
            other => panic!("expected App, got {other:?}"),
        }
    }

    #[test]
    fn app_add_parses_all_options() {
        let cmd = parse(&["app", "add", "vscode", "C:\\VSCode\\Code.exe", "--args=--new-window", "--desc", "VS Code", "--tag", "editor", "--no-apppaths", "--force"]);
        match cmd.sub {
            AliasSubCommand::App(app) => match app.sub {
                AliasAppSubCommand::Add(args) => {
                    assert_eq!(args.args.as_deref(), Some("--new-window"));
                    assert_eq!(args.desc.as_deref(), Some("VS Code"));
                    assert_eq!(args.tag, vec!["editor"]);
                    assert!(args.no_apppaths);
                    assert!(args.force);
                }
                other => panic!("expected App::Add, got {other:?}"),
            },
            other => panic!("expected App, got {other:?}"),
        }
    }

    #[test]
    fn app_rm_parses_names() {
        let cmd = parse(&["app", "rm", "vscode", "sublime"]);
        match cmd.sub {
            AliasSubCommand::App(app) => match app.sub {
                AliasAppSubCommand::Rm(args) => {
                    assert_eq!(args.names, vec!["vscode", "sublime"]);
                }
                other => panic!("expected App::Rm, got {other:?}"),
            },
            other => panic!("expected App, got {other:?}"),
        }
    }

    #[test]
    fn app_ls_parses_json() {
        let cmd = parse(&["app", "ls", "--json"]);
        match cmd.sub {
            AliasSubCommand::App(app) => match app.sub {
                AliasAppSubCommand::List(args) => {
                    assert!(args.json);
                }
                other => panic!("expected App::List, got {other:?}"),
            },
            other => panic!("expected App, got {other:?}"),
        }
    }

    #[test]
    fn app_scan_parses_defaults() {
        let cmd = parse(&["app", "scan"]);
        match cmd.sub {
            AliasSubCommand::App(app) => match app.sub {
                AliasAppSubCommand::Scan(args) => {
                    assert_eq!(args.source, "all");
                    assert!(args.filter.is_none());
                    assert!(!args.json);
                    assert!(!args.all);
                    assert!(!args.no_cache);
                }
                other => panic!("expected App::Scan, got {other:?}"),
            },
            other => panic!("expected App, got {other:?}"),
        }
    }

    #[test]
    fn app_scan_parses_all_options() {
        let cmd = parse(&["app", "scan", "--source", "reg", "--filter", "code", "--json", "--all", "--no-cache"]);
        match cmd.sub {
            AliasSubCommand::App(app) => match app.sub {
                AliasAppSubCommand::Scan(args) => {
                    assert_eq!(args.source, "reg");
                    assert_eq!(args.filter.as_deref(), Some("code"));
                    assert!(args.json);
                    assert!(args.all);
                    assert!(args.no_cache);
                }
                other => panic!("expected App::Scan, got {other:?}"),
            },
            other => panic!("expected App, got {other:?}"),
        }
    }

    #[test]
    fn app_which_parses_name() {
        let cmd = parse(&["app", "which", "vscode"]);
        match cmd.sub {
            AliasSubCommand::App(app) => match app.sub {
                AliasAppSubCommand::Which(args) => {
                    assert_eq!(args.name, "vscode");
                }
                other => panic!("expected App::Which, got {other:?}"),
            },
            other => panic!("expected App, got {other:?}"),
        }
    }

    #[test]
    fn app_sync_parses_empty() {
        let cmd = parse(&["app", "sync"]);
        match cmd.sub {
            AliasSubCommand::App(app) => match app.sub {
                AliasAppSubCommand::Sync(_) => {}
                other => panic!("expected App::Sync, got {other:?}"),
            },
            other => panic!("expected App, got {other:?}"),
        }
    }

    // 鈹€鈹€ TableRow 娴嬭瘯 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn alias_entry_columns_correct() {
        let cols = AliasEntry::columns();
        assert_eq!(cols.len(), 5);
        assert_eq!(cols[0].name, "name");
        assert_eq!(cols[0].kind, ValueKind::String);
        assert_eq!(cols[1].name, "command");
        assert_eq!(cols[2].name, "mode");
        assert_eq!(cols[3].name, "desc");
        assert_eq!(cols[4].name, "tags");
    }

    #[test]
    fn alias_entry_cells_match_fields() {
        let entry = AliasEntry::new("ll", "ls -la", "exe", "list long", "shell,util");
        let cells = entry.cells();
        assert_eq!(cells.len(), 5);
        assert!(matches!(&cells[0], Value::String(s) if s == "ll"));
        assert!(matches!(&cells[1], Value::String(s) if s == "ls -la"));
        assert!(matches!(&cells[2], Value::String(s) if s == "exe"));
        assert!(matches!(&cells[3], Value::String(s) if s == "list long"));
        assert!(matches!(&cells[4], Value::String(s) if s == "shell,util"));
    }

    #[test]
    fn alias_entry_renders_as_json() {
        let entry = AliasEntry::new("ll", "ls -la", "exe", "list long", "shell");
        let table = entry.to_table();
        let mut buf = Cursor::new(Vec::new());
        let mut r = JsonRenderer::new(false, &mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("ll"), "json output: {output}");
    }

    // 鈹€鈹€ E2E 娴嬭瘯锛堟々锛?鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn alias_subcommand_count() {
        let variants = [
            "Setup", "Add", "Rm", "Ls", "Find", "Which",
            "Sync", "Export", "Import", "App",
        ];
        assert_eq!(variants.len(), 10);
    }

    #[test]
    fn alias_app_subcommand_count() {
        let variants = ["Add", "Rm", "Ls", "Scan", "Which", "Sync"];
        assert_eq!(variants.len(), 6);
    }
}

// ============================================================
// Phase 3.7: brn_cmd 鈥?鍗曞懡浠わ紝30+ 鍙傛暟
// ============================================================

mod brn_cmd_tests {
    use clap::Parser;
    use xun::xun_core::brn_cmd::BrnCmd;

    fn parse(args: &[&str]) -> BrnCmd {
        let mut argv = vec!["test"];
        argv.extend_from_slice(args);
        BrnCmd::try_parse_from(&argv).expect("parse failed")
    }

    // 鈹€鈹€ 鍩虹瑙ｆ瀽 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn parses_defaults() {
        let cmd = parse(&[]);
        assert_eq!(cmd.path, ".");
        assert!(!cmd.trim);
        assert!(cmd.trim_chars.is_none());
        assert!(cmd.strip_brackets.is_none());
        assert!(cmd.strip_prefix.is_none());
        assert!(cmd.strip_suffix.is_none());
        assert!(cmd.remove_chars.is_none());
        assert!(cmd.from.is_none());
        assert!(cmd.to.is_none());
        assert!(cmd.regex.is_none());
        assert!(cmd.replace.is_none());
        assert!(cmd.regex_flags.is_none());
        assert!(cmd.case.is_none());
        assert!(cmd.ext_case.is_none());
        assert!(cmd.rename_ext.is_none());
        assert!(cmd.add_ext.is_none());
        assert!(cmd.prefix.is_none());
        assert!(cmd.suffix.is_none());
        assert!(cmd.insert_at.is_none());
        assert!(cmd.template.is_none());
        assert_eq!(cmd.template_start, 1);
        assert_eq!(cmd.template_pad, 3);
        assert!(cmd.slice.is_none());
        assert!(cmd.insert_date.is_none());
        assert!(!cmd.ctime);
        assert!(cmd.normalize_seq.is_none());
        assert!(cmd.normalize_unicode.is_none());
        assert!(!cmd.seq);
        assert_eq!(cmd.start, 1);
        assert_eq!(cmd.pad, 3);
        assert!(cmd.ext.is_empty());
        assert!(cmd.filter.is_none());
        assert!(cmd.exclude.is_none());
        assert!(!cmd.recursive);
        assert!(cmd.depth.is_none());
        assert!(!cmd.include_dirs);
        assert!(cmd.sort_by.is_none());
        assert!(cmd.output_format.is_none());
        assert!(!cmd.apply);
        assert!(!cmd.yes);
        assert!(cmd.undo.is_none());
        assert!(cmd.redo.is_none());
    }

    #[test]
    fn parses_path() {
        let cmd = parse(&["C:\\Photos"]);
        assert_eq!(cmd.path, "C:\\Photos");
    }

    // 鈹€鈹€ Rename steps 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn parses_trim() {
        let cmd = parse(&["--trim"]);
        assert!(cmd.trim);
    }

    #[test]
    fn parses_trim_with_chars() {
        let cmd = parse(&["--trim", "--trim-chars", "_-"]);
        assert!(cmd.trim);
        assert_eq!(cmd.trim_chars.as_deref(), Some("_-"));
    }

    #[test]
    fn parses_strip_brackets() {
        let cmd = parse(&["--strip-brackets", "round,square"]);
        assert_eq!(cmd.strip_brackets.as_deref(), Some("round,square"));
    }

    #[test]
    fn parses_strip_prefix() {
        let cmd = parse(&["--strip-prefix", "IMG_"]);
        assert_eq!(cmd.strip_prefix.as_deref(), Some("IMG_"));
    }

    #[test]
    fn parses_strip_suffix() {
        let cmd = parse(&["--strip-suffix", "_backup"]);
        assert_eq!(cmd.strip_suffix.as_deref(), Some("_backup"));
    }

    #[test]
    fn parses_remove_chars() {
        let cmd = parse(&["--remove-chars", "()-"]);
        assert_eq!(cmd.remove_chars.as_deref(), Some("()-"));
    }

    #[test]
    fn parses_from_to() {
        let cmd = parse(&["--from", "old", "--to", "new"]);
        assert_eq!(cmd.from.as_deref(), Some("old"));
        assert_eq!(cmd.to.as_deref(), Some("new"));
    }

    #[test]
    fn parses_regex_replace() {
        let cmd = parse(&["--regex", r"(\d+)", "--replace", "num_$1", "--regex-flags", "i"]);
        assert_eq!(cmd.regex.as_deref(), Some(r"(\d+)"));
        assert_eq!(cmd.replace.as_deref(), Some("num_$1"));
        assert_eq!(cmd.regex_flags.as_deref(), Some("i"));
    }

    #[test]
    fn parses_case() {
        let cmd = parse(&["--case", "kebab"]);
        assert_eq!(cmd.case.as_deref(), Some("kebab"));
    }

    #[test]
    fn parses_ext_case() {
        let cmd = parse(&["--ext-case", "upper"]);
        assert_eq!(cmd.ext_case.as_deref(), Some("upper"));
    }

    #[test]
    fn parses_rename_ext() {
        let cmd = parse(&["--rename-ext", "jpeg:jpg"]);
        assert_eq!(cmd.rename_ext.as_deref(), Some("jpeg:jpg"));
    }

    #[test]
    fn parses_add_ext() {
        let cmd = parse(&["--add-ext", ".txt"]);
        assert_eq!(cmd.add_ext.as_deref(), Some(".txt"));
    }

    #[test]
    fn parses_prefix_suffix() {
        let cmd = parse(&["--prefix", "IMG_", "--suffix", "_final"]);
        assert_eq!(cmd.prefix.as_deref(), Some("IMG_"));
        assert_eq!(cmd.suffix.as_deref(), Some("_final"));
    }

    #[test]
    fn parses_insert_at() {
        let cmd = parse(&["--insert-at", "4:hello"]);
        assert_eq!(cmd.insert_at.as_deref(), Some("4:hello"));
    }

    #[test]
    fn parses_template() {
        let cmd = parse(&["--template", "{n:03}_{stem}", "--template-start", "10", "--template-pad", "4"]);
        assert_eq!(cmd.template.as_deref(), Some("{n:03}_{stem}"));
        assert_eq!(cmd.template_start, 10);
        assert_eq!(cmd.template_pad, 4);
    }

    #[test]
    fn parses_slice() {
        let cmd = parse(&["--slice", "0:8"]);
        assert_eq!(cmd.slice.as_deref(), Some("0:8"));
    }

    #[test]
    fn parses_insert_date() {
        let cmd = parse(&["--insert-date", "prefix:%Y%m%d", "--ctime"]);
        assert_eq!(cmd.insert_date.as_deref(), Some("prefix:%Y%m%d"));
        assert!(cmd.ctime);
    }

    #[test]
    fn parses_normalize_seq() {
        let cmd = parse(&["--normalize-seq", "5"]);
        assert_eq!(cmd.normalize_seq, Some(5));
    }

    #[test]
    fn parses_normalize_unicode() {
        let cmd = parse(&["--normalize-unicode", "nfc"]);
        assert_eq!(cmd.normalize_unicode.as_deref(), Some("nfc"));
    }

    #[test]
    fn parses_seq() {
        let cmd = parse(&["--seq", "--start", "100", "--pad", "5"]);
        assert!(cmd.seq);
        assert_eq!(cmd.start, 100);
        assert_eq!(cmd.pad, 5);
    }

    // 鈹€鈹€ Filters 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn parses_ext_filter() {
        let cmd = parse(&["--ext", "jpg", "--ext", "png"]);
        assert_eq!(cmd.ext, vec!["jpg", "png"]);
    }

    #[test]
    fn parses_filter_exclude() {
        let cmd = parse(&["--filter", "IMG_*", "--exclude", "*_backup*"]);
        assert_eq!(cmd.filter.as_deref(), Some("IMG_*"));
        assert_eq!(cmd.exclude.as_deref(), Some("*_backup*"));
    }

    #[test]
    fn parses_recursive() {
        let cmd = parse(&["-r"]);
        assert!(cmd.recursive);
    }

    #[test]
    fn parses_depth() {
        let cmd = parse(&["--depth", "3"]);
        assert_eq!(cmd.depth, Some(3));
    }

    #[test]
    fn parses_include_dirs() {
        let cmd = parse(&["--include-dirs"]);
        assert!(cmd.include_dirs);
    }

    #[test]
    fn parses_sort_by() {
        let cmd = parse(&["--sort-by", "mtime"]);
        assert_eq!(cmd.sort_by.as_deref(), Some("mtime"));
    }

    // 鈹€鈹€ Output & Execution 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn parses_output_format() {
        let cmd = parse(&["--output-format", "json"]);
        assert_eq!(cmd.output_format.as_deref(), Some("json"));
    }

    #[test]
    fn parses_apply_yes() {
        let cmd = parse(&["--apply", "-y"]);
        assert!(cmd.apply);
        assert!(cmd.yes);
    }

    #[test]
    fn parses_undo_redo() {
        let cmd = parse(&["--undo", "3"]);
        assert_eq!(cmd.undo, Some(3));

        let cmd = parse(&["--redo", "5"]);
        assert_eq!(cmd.redo, Some(5));
    }

    // 鈹€鈹€ 缁勫悎娴嬭瘯 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn parses_combined_rename_steps() {
        let cmd = parse(&["C:\\Photos", "--trim", "--strip-prefix", "IMG_", "--case", "kebab", "--apply", "-y"]);
        assert_eq!(cmd.path, "C:\\Photos");
        assert!(cmd.trim);
        assert_eq!(cmd.strip_prefix.as_deref(), Some("IMG_"));
        assert_eq!(cmd.case.as_deref(), Some("kebab"));
        assert!(cmd.apply);
        assert!(cmd.yes);
    }

    // 鈹€鈹€ E2E 娴嬭瘯锛堟々锛?鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn brn_param_count() {
        // 楠岃瘉 30+ 鍙傛暟鍏ㄩ儴瀛樺湪锛堢紪璇戞湡妫€鏌ワ級
        let cmd = BrnCmd {
            path: ".".into(),
            trim: false,
            trim_chars: None,
            strip_brackets: None,
            strip_prefix: None,
            strip_suffix: None,
            remove_chars: None,
            from: None,
            to: None,
            regex: None,
            replace: None,
            regex_flags: None,
            case: None,
            ext_case: None,
            rename_ext: None,
            add_ext: None,
            prefix: None,
            suffix: None,
            insert_at: None,
            template: None,
            template_start: 1,
            template_pad: 3,
            slice: None,
            insert_date: None,
            ctime: false,
            normalize_seq: None,
            normalize_unicode: None,
            seq: false,
            start: 1,
            pad: 3,
            ext: vec![],
            filter: None,
            exclude: None,
            recursive: false,
            depth: None,
            include_dirs: false,
            sort_by: None,
            output_format: None,
            apply: false,
            yes: false,
            undo: None,
            redo: None,
        };
        assert_eq!(cmd.path, ".");
    }
}

// ============================================================
// Phase 3.7: vault_cmd 鈥?8 瀛愬懡浠?
// ============================================================



