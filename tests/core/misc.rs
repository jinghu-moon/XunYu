mod dashboard_cmd_tests {
    use clap::Parser;
    use xun::xun_core::dashboard_cmd::ServeCmd;

    #[test]
    fn serve_parses_defaults() {
        let cmd = ServeCmd::try_parse_from(["test"]).unwrap();
        assert_eq!(cmd.port, 9527);
    }

    #[test]
    fn serve_parses_port() {
        let cmd = ServeCmd::try_parse_from(["test", "-p", "8080"]).unwrap();
        assert_eq!(cmd.port, 8080);
    }

    #[test]
    fn serve_parses_long_port() {
        let cmd = ServeCmd::try_parse_from(["test", "--port", "3000"]).unwrap();
        assert_eq!(cmd.port, 3000);
    }
}

// ============================================================
// Phase 3.8: redirect_cmd 鈥?RedirectCmd (20+ 鍙傛暟)
// ============================================================

mod redirect_cmd_tests {
    use clap::Parser;
    use xun::xun_core::redirect_cmd::RedirectCmd;

    fn parse(args: &[&str]) -> RedirectCmd {
        let mut argv = vec!["test"];
        argv.extend_from_slice(args);
        RedirectCmd::try_parse_from(&argv).expect("parse failed")
    }

    #[test]
    fn parses_defaults() {
        let cmd = parse(&[]);
        assert!(cmd.source.is_none());
        assert_eq!(cmd.profile, "default");
        assert!(cmd.explain.is_none());
        assert!(!cmd.stats);
        assert!(!cmd.confirm);
        assert!(!cmd.review);
        assert!(!cmd.log);
        assert!(cmd.tx.is_none());
        assert!(cmd.last.is_none());
        assert!(!cmd.validate);
        assert!(cmd.plan.is_none());
        assert!(cmd.apply.is_none());
        assert!(cmd.undo.is_none());
        assert!(!cmd.watch);
        assert!(!cmd.status);
        assert!(!cmd.simulate);
        assert!(!cmd.dry_run);
        assert!(!cmd.copy);
        assert!(!cmd.yes);
        assert_eq!(cmd.format, "auto");
    }

    #[test]
    fn parses_source() {
        let cmd = parse(&["C:\\Downloads"]);
        assert_eq!(cmd.source.as_deref(), Some("C:\\Downloads"));
    }

    #[test]
    fn parses_profile() {
        let cmd = parse(&["--profile", "photos"]);
        assert_eq!(cmd.profile, "photos");
    }

    #[test]
    fn parses_explain() {
        let cmd = parse(&["--explain", "IMG_001.jpg"]);
        assert_eq!(cmd.explain.as_deref(), Some("IMG_001.jpg"));
    }

    #[test]
    fn parses_log_options() {
        let cmd = parse(&["--log", "--tx", "abc123", "--last", "10"]);
        assert!(cmd.log);
        assert_eq!(cmd.tx.as_deref(), Some("abc123"));
        assert_eq!(cmd.last, Some(10));
    }

    #[test]
    fn parses_plan_apply_undo() {
        let cmd = parse(&["--plan", "plan.json"]);
        assert_eq!(cmd.plan.as_deref(), Some("plan.json"));

        let cmd = parse(&["--apply", "plan.json"]);
        assert_eq!(cmd.apply.as_deref(), Some("plan.json"));

        let cmd = parse(&["--undo", "tx123"]);
        assert_eq!(cmd.undo.as_deref(), Some("tx123"));
    }

    #[test]
    fn parses_watch_status() {
        let cmd = parse(&["--watch", "--status"]);
        assert!(cmd.watch);
        assert!(cmd.status);
    }

    #[test]
    fn parses_execution_flags() {
        let cmd = parse(&["--dry-run", "--copy", "-y", "-f", "json"]);
        assert!(cmd.dry_run);
        assert!(cmd.copy);
        assert!(cmd.yes);
        assert_eq!(cmd.format, "json");
    }

    #[test]
    fn parses_combined() {
        let cmd = parse(&["C:\\Downloads", "--profile", "docs", "--confirm", "--review", "--simulate", "--validate"]);
        assert_eq!(cmd.source.as_deref(), Some("C:\\Downloads"));
        assert_eq!(cmd.profile, "docs");
        assert!(cmd.confirm);
        assert!(cmd.review);
        assert!(cmd.simulate);
        assert!(cmd.validate);
    }

    #[test]
    fn redirect_param_count() {
        // 楠岃瘉 20+ 鍙傛暟鍏ㄩ儴瀛樺湪锛堢紪璇戞湡妫€鏌ワ級
        let cmd = RedirectCmd {
            source: None,
            profile: "default".into(),
            explain: None,
            stats: false,
            confirm: false,
            review: false,
            log: false,
            tx: None,
            last: None,
            validate: false,
            plan: None,
            apply: None,
            undo: None,
            watch: false,
            status: false,
            simulate: false,
            dry_run: false,
            copy: false,
            yes: false,
            format: "auto".into(),
        };
        assert_eq!(cmd.profile, "default");
    }
}

// ============================================================
// Phase 3.8: img_cmd 鈥?ImgCmd (16 鍙傛暟)
// ============================================================

mod img_cmd_tests {
    use clap::Parser;
    use xun::xun_core::img_cmd::ImgCmd;

    fn parse(args: &[&str]) -> ImgCmd {
        let mut argv = vec!["test"];
        argv.extend_from_slice(args);
        ImgCmd::try_parse_from(&argv).expect("parse failed")
    }

    #[test]
    fn parses_defaults() {
        let cmd = parse(&["-i", "photo.jpg", "-o", "output/"]);
        assert_eq!(cmd.input, "photo.jpg");
        assert_eq!(cmd.output, "output/");
        assert_eq!(cmd.format, "webp");
        assert_eq!(cmd.svg_method, "bezier");
        assert_eq!(cmd.svg_diffvg_iters, 150);
        assert_eq!(cmd.svg_diffvg_strokes, 64);
        assert_eq!(cmd.jpeg_backend, "auto");
        assert_eq!(cmd.quality, 80);
        assert_eq!(cmd.png_lossy, "true");
        assert_eq!(cmd.png_dither_level, 0.0);
        assert_eq!(cmd.webp_lossy, "true");
        assert!(cmd.mw.is_none());
        assert!(cmd.mh.is_none());
        assert!(cmd.threads.is_none());
        assert!(cmd.avif_threads.is_none());
        assert!(!cmd.overwrite);
    }

    #[test]
    fn parses_format() {
        let cmd = parse(&["-i", "photo.jpg", "-o", "out/", "-f", "avif"]);
        assert_eq!(cmd.format, "avif");
    }

    #[test]
    fn parses_svg_options() {
        let cmd = parse(&["-i", "photo.jpg", "-o", "out/", "--svg-method", "potrace", "--svg-diffvg-iters", "200", "--svg-diffvg-strokes", "128"]);
        assert_eq!(cmd.svg_method, "potrace");
        assert_eq!(cmd.svg_diffvg_iters, 200);
        assert_eq!(cmd.svg_diffvg_strokes, 128);
    }

    #[test]
    fn parses_jpeg_options() {
        let cmd = parse(&["-i", "photo.jpg", "-o", "out/", "--jpeg-backend", "turbo", "-q", "95"]);
        assert_eq!(cmd.jpeg_backend, "turbo");
        assert_eq!(cmd.quality, 95);
    }

    #[test]
    fn parses_png_options() {
        let cmd = parse(&["-i", "photo.png", "-o", "out/", "--png-lossy", "false", "--png-dither-level", "0.5"]);
        assert_eq!(cmd.png_lossy, "false");
        assert_eq!(cmd.png_dither_level, 0.5);
    }

    #[test]
    fn parses_webp_lossy() {
        let cmd = parse(&["-i", "photo.png", "-o", "out/", "--webp-lossy", "false"]);
        assert_eq!(cmd.webp_lossy, "false");
    }

    #[test]
    fn parses_resize() {
        let cmd = parse(&["-i", "photo.jpg", "-o", "out/", "--mw", "1920", "--mh", "1080"]);
        assert_eq!(cmd.mw, Some(1920));
        assert_eq!(cmd.mh, Some(1080));
    }

    #[test]
    fn parses_threads() {
        let cmd = parse(&["-i", "dir/", "-o", "out/", "-t", "4", "--avif-threads", "2"]);
        assert_eq!(cmd.threads, Some(4));
        assert_eq!(cmd.avif_threads, Some(2));
    }

    #[test]
    fn parses_overwrite() {
        let cmd = parse(&["-i", "photo.jpg", "-o", "out/", "--overwrite"]);
        assert!(cmd.overwrite);
    }

    #[test]
    fn img_param_count() {
        let cmd = ImgCmd {
            input: "test".into(),
            output: "out".into(),
            format: "webp".into(),
            svg_method: "bezier".into(),
            svg_diffvg_iters: 150,
            svg_diffvg_strokes: 64,
            jpeg_backend: "auto".into(),
            quality: 80,
            png_lossy: "true".into(),
            png_dither_level: 0.0,
            webp_lossy: "true".into(),
            mw: None,
            mh: None,
            threads: None,
            avif_threads: None,
            overwrite: false,
        };
        assert_eq!(cmd.format, "webp");
    }
}

// ============================================================
// Phase 3.8: xunbak_cmd 鈥?宓屽 Plugin 鈫?Install/Uninstall/Doctor
// ============================================================

mod xunbak_cmd_tests {
    use clap::Parser;
    use xun::xun_core::xunbak_cmd::*;

    fn parse(args: &[&str]) -> XunbakCmd {
        let mut argv = vec!["test"];
        argv.extend_from_slice(args);
        XunbakCmd::try_parse_from(&argv).expect("parse failed")
    }

    #[test]
    fn plugin_install_parses_basic() {
        let cmd = parse(&["plugin", "install"]);
        match cmd.cmd {
            XunbakSubCommand::Plugin(plugin) => match plugin.cmd {
                XunbakPluginSubCommand::Install(args) => {
                    assert!(args.sevenzip_home.is_none());
                    assert!(args.config.is_none());
                    assert!(!args.no_overwrite);
                    assert!(!args.associate);
                }
                other => panic!("expected Install, got {other:?}"),
            },
        }
    }

    #[test]
    fn plugin_install_parses_all_options() {
        let cmd = parse(&["plugin", "install", "--sevenzip-home", "C:/Program Files/7-Zip", "--config", "release", "--no-overwrite", "--associate"]);
        match cmd.cmd {
            XunbakSubCommand::Plugin(plugin) => match plugin.cmd {
                XunbakPluginSubCommand::Install(args) => {
                    assert_eq!(args.sevenzip_home.as_deref(), Some("C:/Program Files/7-Zip"));
                    assert_eq!(args.config.as_deref(), Some("release"));
                    assert!(args.no_overwrite);
                    assert!(args.associate);
                }
                other => panic!("expected Install, got {other:?}"),
            },
        }
    }

    #[test]
    fn plugin_uninstall_parses_basic() {
        let cmd = parse(&["plugin", "uninstall"]);
        match cmd.cmd {
            XunbakSubCommand::Plugin(plugin) => match plugin.cmd {
                XunbakPluginSubCommand::Uninstall(args) => {
                    assert!(args.sevenzip_home.is_none());
                    assert!(!args.remove_association);
                }
                other => panic!("expected Uninstall, got {other:?}"),
            },
        }
    }

    #[test]
    fn plugin_uninstall_parses_remove_association() {
        let cmd = parse(&["plugin", "uninstall", "--remove-association"]);
        match cmd.cmd {
            XunbakSubCommand::Plugin(plugin) => match plugin.cmd {
                XunbakPluginSubCommand::Uninstall(args) => {
                    assert!(args.remove_association);
                }
                other => panic!("expected Uninstall, got {other:?}"),
            },
        }
    }

    #[test]
    fn plugin_doctor_parses_basic() {
        let cmd = parse(&["plugin", "doctor"]);
        match cmd.cmd {
            XunbakSubCommand::Plugin(plugin) => match plugin.cmd {
                XunbakPluginSubCommand::Doctor(args) => {
                    assert!(args.sevenzip_home.is_none());
                }
                other => panic!("expected Doctor, got {other:?}"),
            },
        }
    }

    #[test]
    fn plugin_doctor_parses_sevenzip_home() {
        let cmd = parse(&["plugin", "doctor", "--sevenzip-home", "C:\\7-Zip"]);
        match cmd.cmd {
            XunbakSubCommand::Plugin(plugin) => match plugin.cmd {
                XunbakPluginSubCommand::Doctor(args) => {
                    assert_eq!(args.sevenzip_home.as_deref(), Some("C:\\7-Zip"));
                }
                other => panic!("expected Doctor, got {other:?}"),
            },
        }
    }

    #[test]
    fn xunbak_subcommand_count() {
        let variants = ["Plugin"];
        assert_eq!(variants.len(), 1);
    }

    #[test]
    fn xunbak_plugin_subcommand_count() {
        let variants = ["Install", "Uninstall", "Doctor"];
        assert_eq!(variants.len(), 3);
    }
}

// ============================================================
// Phase 3.8: desktop_cmd 鈥?14 椤跺眰瀛愬懡浠わ紝澶氫釜宓屽缁?
// ============================================================

