use super::common::*;
use crate::common::*;

#[test]
fn acl_add_batch_file_writes_audit() {
    let env = TestEnv::new();
    let dirs = vec![
        setup_acl_dir(&env, "acl_add_batch_1"),
        setup_acl_dir(&env, "acl_add_batch_2"),
        setup_acl_dir(&env, "acl_add_batch_3"),
    ];

    let list_path = env.root.join("acl_add_batch.txt");
    let content: String = dirs
        .iter()
        .map(|p| {
            format!(
                "{}
",
                str_path(p)
            )
        })
        .collect();
    std::fs::write(&list_path, content).unwrap();

    run_ok(acl_cmd(&env).args([
        "acl",
        "add",
        "--file",
        &str_path(&list_path),
        "--principal",
        "BUILTIN\\Users",
        "--rights",
        "Read",
        "--ace-type",
        "Allow",
        "--inherit",
        "None",
        "-y",
    ]));

    let add_paths = read_audit_paths_for_action(&env, "AddPermission");
    assert_eq!(
        add_paths.len(),
        dirs.len(),
        "expected one audit entry per path"
    );
    for path in &dirs {
        let path_str = str_path(path);
        assert!(
            add_paths.iter().any(|p| p == &path_str),
            "missing audit entry for {path_str}"
        );
    }
}

#[test]
fn acl_add_batch_with_missing_path_reports_error() {
    let env = TestEnv::new();
    let dirs = vec![
        setup_acl_dir(&env, "acl_add_batch_ok_1"),
        setup_acl_dir(&env, "acl_add_batch_ok_2"),
    ];
    let missing = env.root.join("acl_add_batch_missing");

    let list_path = env.root.join("acl_add_batch_missing.txt");
    let mut content: String = dirs
        .iter()
        .map(|p| {
            format!(
                "{}
",
                str_path(p)
            )
        })
        .collect();
    content.push_str(&format!(
        "{}
",
        str_path(&missing)
    ));
    std::fs::write(&list_path, content).unwrap();

    let out = run_err(acl_cmd(&env).args([
        "acl",
        "add",
        "--file",
        &str_path(&list_path),
        "--principal",
        "BUILTIN\\Users",
        "--rights",
        "Read",
        "--ace-type",
        "Allow",
        "--inherit",
        "None",
        "-y",
    ]));
    let err = stderr_str(&out);
    assert!(
        err.contains("Invalid path input.")
            || err.contains("Invalid path")
            || err.contains("Batch failed")
            || err.contains("failed"),
        "expected batch failure message: {err}"
    );

    let add_paths = read_audit_paths_for_action(&env, "AddPermission");
    assert!(
        add_paths.is_empty(),
        "expected no audit entries when validation fails"
    );
}

#[test]
fn acl_add_batch_parallel_audit_consistency() {
    let env = TestEnv::new();
    let mut dirs = Vec::new();
    for i in 0..32 {
        dirs.push(setup_acl_dir(&env, &format!("acl_add_batch_par_{i}")));
    }

    let list_path = env.root.join("acl_add_batch_par.txt");
    let content: String = dirs
        .iter()
        .map(|p| {
            format!(
                "{}
",
                str_path(p)
            )
        })
        .collect();
    std::fs::write(&list_path, content).unwrap();

    run_ok(acl_cmd(&env).args([
        "acl",
        "add",
        "--file",
        &str_path(&list_path),
        "--principal",
        "BUILTIN\\Users",
        "--rights",
        "Read",
        "--ace-type",
        "Allow",
        "--inherit",
        "None",
        "-y",
    ]));

    let add_paths = read_audit_paths_for_action(&env, "AddPermission");
    assert_eq!(
        add_paths.len(),
        dirs.len(),
        "expected audit entries for all paths"
    );
    for path in &dirs {
        let path_str = str_path(path);
        assert!(
            add_paths.iter().any(|p| p == &path_str),
            "missing audit entry for {path_str}"
        );
    }
}
