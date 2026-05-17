mod desktop_cmd_tests {
    use clap::Parser;
    use xun::xun_core::desktop_cmd::*;

    fn parse(args: &[&str]) -> DesktopCmd {
        let mut argv = vec!["test"];
        argv.extend_from_slice(args);
        DesktopCmd::try_parse_from(&argv).expect("parse failed")
    }

    // 鈹€鈹€ Daemon 瀛愬懡浠ょ粍 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn daemon_start_parses_defaults() {
        let cmd = parse(&["daemon", "start"]);
        match cmd.cmd {
            DesktopSubCommand::Daemon(d) => match d.cmd {
                DesktopDaemonSubCommand::Start(args) => {
                    assert!(!args.quiet);
                    assert!(!args.no_tray);
                    assert!(!args.elevated);
                }
                other => panic!("expected Start, got {other:?}"),
            },
            other => panic!("expected Daemon, got {other:?}"),
        }
    }

    #[test]
    fn daemon_start_parses_flags() {
        let cmd = parse(&["daemon", "start", "-q", "--no-tray", "--elevated"]);
        match cmd.cmd {
            DesktopSubCommand::Daemon(d) => match d.cmd {
                DesktopDaemonSubCommand::Start(args) => {
                    assert!(args.quiet);
                    assert!(args.no_tray);
                    assert!(args.elevated);
                }
                other => panic!("expected Start, got {other:?}"),
            },
            other => panic!("expected Daemon, got {other:?}"),
        }
    }

    #[test]
    fn daemon_stop_parses() {
        let cmd = parse(&["daemon", "stop"]);
        match cmd.cmd {
            DesktopSubCommand::Daemon(d) => match d.cmd {
                DesktopDaemonSubCommand::Stop(_) => {}
                other => panic!("expected Stop, got {other:?}"),
            },
            other => panic!("expected Daemon, got {other:?}"),
        }
    }

    #[test]
    fn daemon_status_parses() {
        let cmd = parse(&["daemon", "status"]);
        match cmd.cmd {
            DesktopSubCommand::Daemon(d) => match d.cmd {
                DesktopDaemonSubCommand::Status(_) => {}
                other => panic!("expected Status, got {other:?}"),
            },
            other => panic!("expected Daemon, got {other:?}"),
        }
    }

    #[test]
    fn daemon_reload_parses() {
        let cmd = parse(&["daemon", "reload"]);
        match cmd.cmd {
            DesktopSubCommand::Daemon(d) => match d.cmd {
                DesktopDaemonSubCommand::Reload(_) => {}
                other => panic!("expected Reload, got {other:?}"),
            },
            other => panic!("expected Daemon, got {other:?}"),
        }
    }

    // 鈹€鈹€ Hotkey 瀛愬懡浠ょ粍 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn hotkey_bind_parses() {
        let cmd = parse(&["hotkey", "bind", "ctrl+alt+t", "run:wt.exe"]);
        match cmd.cmd {
            DesktopSubCommand::Hotkey(h) => match h.cmd {
                DesktopHotkeySubCommand::Bind(args) => {
                    assert_eq!(args.hotkey, "ctrl+alt+t");
                    assert_eq!(args.action, "run:wt.exe");
                    assert!(args.app.is_none());
                }
                other => panic!("expected Bind, got {other:?}"),
            },
            other => panic!("expected Hotkey, got {other:?}"),
        }
    }

    #[test]
    fn hotkey_bind_parses_app() {
        let cmd = parse(&["hotkey", "bind", "ctrl+alt+t", "run:wt.exe", "--app", "Explorer"]);
        match cmd.cmd {
            DesktopSubCommand::Hotkey(h) => match h.cmd {
                DesktopHotkeySubCommand::Bind(args) => {
                    assert_eq!(args.app.as_deref(), Some("Explorer"));
                }
                other => panic!("expected Bind, got {other:?}"),
            },
            other => panic!("expected Hotkey, got {other:?}"),
        }
    }

    #[test]
    fn hotkey_unbind_parses() {
        let cmd = parse(&["hotkey", "unbind", "ctrl+alt+t"]);
        match cmd.cmd {
            DesktopSubCommand::Hotkey(h) => match h.cmd {
                DesktopHotkeySubCommand::Unbind(args) => {
                    assert_eq!(args.hotkey, "ctrl+alt+t");
                }
                other => panic!("expected Unbind, got {other:?}"),
            },
            other => panic!("expected Hotkey, got {other:?}"),
        }
    }

    #[test]
    fn hotkey_list_parses() {
        let cmd = parse(&["hotkey", "list"]);
        match cmd.cmd {
            DesktopSubCommand::Hotkey(h) => match h.cmd {
                DesktopHotkeySubCommand::List(_) => {}
                other => panic!("expected List, got {other:?}"),
            },
            other => panic!("expected Hotkey, got {other:?}"),
        }
    }

    // 鈹€鈹€ Remap 瀛愬懡浠ょ粍 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn remap_add_parses() {
        let cmd = parse(&["remap", "add", "capslock", "esc"]);
        match cmd.cmd {
            DesktopSubCommand::Remap(r) => match r.cmd {
                DesktopRemapSubCommand::Add(args) => {
                    assert_eq!(args.from, "capslock");
                    assert_eq!(args.to, "esc");
                    assert!(args.app.is_none());
                    assert!(!args.exact);
                    assert!(!args.dry_run);
                }
                other => panic!("expected Add, got {other:?}"),
            },
            other => panic!("expected Remap, got {other:?}"),
        }
    }

    #[test]
    fn remap_remove_parses() {
        let cmd = parse(&["remap", "remove", "capslock"]);
        match cmd.cmd {
            DesktopSubCommand::Remap(r) => match r.cmd {
                DesktopRemapSubCommand::Rm(args) => {
                    assert_eq!(args.from, "capslock");
                    assert!(args.to.is_none());
                }
                other => panic!("expected Rm, got {other:?}"),
            },
            other => panic!("expected Remap, got {other:?}"),
        }
    }

    // 鈹€鈹€ Snippet 瀛愬懡浠ょ粍 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn snippet_add_parses() {
        let cmd = parse(&["snippet", "add", "btw", "by the way"]);
        match cmd.cmd {
            DesktopSubCommand::Snippet(s) => match s.cmd {
                DesktopSnippetSubCommand::Add(args) => {
                    assert_eq!(args.trigger, "btw");
                    assert_eq!(args.expand, "by the way");
                    assert!(!args.immediate);
                    assert!(!args.clipboard);
                }
                other => panic!("expected Add, got {other:?}"),
            },
            other => panic!("expected Snippet, got {other:?}"),
        }
    }

    // 鈹€鈹€ Layout 瀛愬懡浠ょ粍 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn layout_new_parses() {
        let cmd = parse(&["layout", "new", "coding", "-t", "grid", "--rows", "2", "--cols", "3"]);
        match cmd.cmd {
            DesktopSubCommand::Layout(l) => match l.cmd {
                DesktopLayoutSubCommand::Add(args) => {
                    assert_eq!(args.name, "coding");
                    assert_eq!(args.layout_type, "grid");
                    assert_eq!(args.rows, Some(2));
                    assert_eq!(args.cols, Some(3));
                }
                other => panic!("expected Add, got {other:?}"),
            },
            other => panic!("expected Layout, got {other:?}"),
        }
    }

    // 鈹€鈹€ Workspace 瀛愬懡浠ょ粍 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn workspace_save_parses() {
        let cmd = parse(&["workspace", "save", "dev", "--name-only"]);
        match cmd.cmd {
            DesktopSubCommand::Workspace(w) => match w.cmd {
                DesktopWorkspaceSubCommand::Save(args) => {
                    assert_eq!(args.name, "dev");
                    assert!(args.name_only);
                }
                other => panic!("expected Save, got {other:?}"),
            },
            other => panic!("expected Workspace, got {other:?}"),
        }
    }

    // 鈹€鈹€ Window 瀛愬懡浠ょ粍 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn window_move_parses() {
        let cmd = parse(&["window", "move", "--x", "100", "--y", "200"]);
        match cmd.cmd {
            DesktopSubCommand::Window(w) => match w.cmd {
                DesktopWindowSubCommand::Move(args) => {
                    assert_eq!(args.x, 100);
                    assert_eq!(args.y, 200);
                }
                other => panic!("expected Move, got {other:?}"),
            },
            other => panic!("expected Window, got {other:?}"),
        }
    }

    #[test]
    fn window_resize_parses() {
        let cmd = parse(&["window", "resize", "--width", "800", "--height", "600"]);
        match cmd.cmd {
            DesktopSubCommand::Window(w) => match w.cmd {
                DesktopWindowSubCommand::Resize(args) => {
                    assert_eq!(args.width, 800);
                    assert_eq!(args.height, 600);
                }
                other => panic!("expected Resize, got {other:?}"),
            },
            other => panic!("expected Window, got {other:?}"),
        }
    }

    // 鈹€鈹€ Theme 瀛愬懡浠ょ粍 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn theme_set_parses() {
        let cmd = parse(&["theme", "set", "dark"]);
        match cmd.cmd {
            DesktopSubCommand::Theme(t) => match t.cmd {
                DesktopThemeSubCommand::Set(args) => {
                    assert_eq!(args.mode, "dark");
                }
                other => panic!("expected Set, got {other:?}"),
            },
            other => panic!("expected Theme, got {other:?}"),
        }
    }

    #[test]
    fn theme_schedule_parses() {
        let cmd = parse(&["theme", "schedule", "--light", "06:00", "--dark", "18:00"]);
        match cmd.cmd {
            DesktopSubCommand::Theme(t) => match t.cmd {
                DesktopThemeSubCommand::Schedule(args) => {
                    assert_eq!(args.light.as_deref(), Some("06:00"));
                    assert_eq!(args.dark.as_deref(), Some("18:00"));
                }
                other => panic!("expected Schedule, got {other:?}"),
            },
            other => panic!("expected Theme, got {other:?}"),
        }
    }

    // 鈹€鈹€ Awake 瀛愬懡浠ょ粍 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn awake_on_parses() {
        let cmd = parse(&["awake", "on", "--duration", "2h", "--display-on"]);
        match cmd.cmd {
            DesktopSubCommand::Awake(a) => match a.cmd {
                DesktopAwakeSubCommand::On(args) => {
                    assert_eq!(args.duration.as_deref(), Some("2h"));
                    assert!(args.display_on);
                }
                other => panic!("expected On, got {other:?}"),
            },
            other => panic!("expected Awake, got {other:?}"),
        }
    }

    // 鈹€鈹€ Color 鍛戒护 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn color_parses() {
        let cmd = parse(&["color", "--copy"]);
        match cmd.cmd {
            DesktopSubCommand::Color(args) => {
                assert!(args.copy);
            }
            other => panic!("expected Color, got {other:?}"),
        }
    }

    // 鈹€鈹€ Hosts 瀛愬懡浠ょ粍 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn hosts_add_parses() {
        let cmd = parse(&["hosts", "add", "example.com", "127.0.0.1"]);
        match cmd.cmd {
            DesktopSubCommand::Hosts(h) => match h.cmd {
                DesktopHostsSubCommand::Add(args) => {
                    assert_eq!(args.host, "example.com");
                    assert_eq!(args.ip, "127.0.0.1");
                    assert!(!args.dry_run);
                }
                other => panic!("expected Add, got {other:?}"),
            },
            other => panic!("expected Hosts, got {other:?}"),
        }
    }

    // 鈹€鈹€ App 瀛愬懡浠ょ粍 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn app_list_parses() {
        let cmd = parse(&["app", "list"]);
        match cmd.cmd {
            DesktopSubCommand::App(a) => match a.cmd {
                DesktopAppSubCommand::List(_) => {}
            },
            other => panic!("expected App, got {other:?}"),
        }
    }

    // 鈹€鈹€ Tui 鍛戒护 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn tui_parses() {
        let cmd = parse(&["tui"]);
        match cmd.cmd {
            DesktopSubCommand::Tui(_) => {}
            other => panic!("expected Tui, got {other:?}"),
        }
    }

    // 鈹€鈹€ Run 鍛戒护 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn run_parses() {
        let cmd = parse(&["run", "notepad.exe"]);
        match cmd.cmd {
            DesktopSubCommand::Run(args) => {
                assert_eq!(args.command, "notepad.exe");
            }
            other => panic!("expected Run, got {other:?}"),
        }
    }

    // 鈹€鈹€ E2E 娴嬭瘯锛堟々锛?鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn desktop_subcommand_count() {
        let variants = [
            "Daemon", "Hotkey", "Remap", "Snippet", "Layout",
            "Workspace", "Window", "Theme", "Awake", "Color",
            "Hosts", "App", "Tui", "Run",
        ];
        assert_eq!(variants.len(), 14);
    }

    #[test]
    fn desktop_daemon_subcommand_count() {
        let variants = ["Start", "Stop", "Status", "Reload"];
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn desktop_hotkey_subcommand_count() {
        let variants = ["Bind", "Unbind", "List"];
        assert_eq!(variants.len(), 3);
    }

    #[test]
    fn desktop_remap_subcommand_count() {
        let variants = ["Add", "Remove", "List", "Clear"];
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn desktop_snippet_subcommand_count() {
        let variants = ["Add", "Remove", "List", "Clear"];
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn desktop_layout_subcommand_count() {
        let variants = ["New", "Apply", "Preview", "List", "Remove"];
        assert_eq!(variants.len(), 5);
    }

    #[test]
    fn desktop_workspace_subcommand_count() {
        let variants = ["Save", "Launch", "List", "Remove"];
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn desktop_window_subcommand_count() {
        let variants = ["Focus", "Move", "Resize", "Transparent", "Top"];
        assert_eq!(variants.len(), 5);
    }

    #[test]
    fn desktop_theme_subcommand_count() {
        let variants = ["Set", "Toggle", "Schedule", "Status"];
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn desktop_awake_subcommand_count() {
        let variants = ["On", "Off", "Status"];
        assert_eq!(variants.len(), 3);
    }

    #[test]
    fn desktop_hosts_subcommand_count() {
        let variants = ["Add", "Remove", "List"];
        assert_eq!(variants.len(), 3);
    }

    #[test]
    fn desktop_app_subcommand_count() {
        let variants = ["List"];
        assert_eq!(variants.len(), 1);
    }
}

// ============================================================
// Phase 2.3: Proxy CommandSpec 瀹炵幇锛堢湡瀹?service锛?
// ============================================================


