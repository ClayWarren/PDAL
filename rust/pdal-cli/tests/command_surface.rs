use std::process::Command;

#[test]
fn root_argument_errors_and_driver_table_run() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--bogus-root-option")
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Unexpected argument"));

    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--drivers")
        .output()
        .unwrap();
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("readers.las"));

    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--label")
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("--label requires"));

    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .args(["--options", "all"])
        .output()
        .unwrap();
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("readers.las"));
}

#[test]
fn unknown_command_fails_cleanly() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("bogus-command")
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("Unknown Rust command 'bogus-command'")
    );
}

#[test]
fn command_local_help_succeeds() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("tindex")
        .arg("--help")
        .output()
        .unwrap();

    assert!(result.status.success());
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("pdal tindex create"));
    assert!(stdout.contains("--filelist"));
}

#[test]
fn list_commands_reports_rust_commands() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--list-commands")
        .output()
        .unwrap();

    assert!(result.status.success());
    assert_eq!(
        String::from_utf8_lossy(&result.stdout),
        "chamfer\ndelta\ndensity\neval\nfauxplugin\nground\nhausdorff\ninfo\nlasdump\nnitfwrap\nmerge\npipeline\nrandom\nsort\nsplit\ntile\ntindex\ntranslate\n"
    );
}

#[test]
fn version_supports_json_native_dependency_report() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("--version")
        .output()
        .unwrap();

    assert!(result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(json["name"], "pdal-rs");
    assert!(json["native_dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .any(|dependency| dependency["name"] == "PROJ"));
}

#[test]
fn list_commands_supports_json() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("--list-commands")
        .output()
        .unwrap();

    assert!(result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(json[0]["name"], "chamfer");
    assert_eq!(json[0]["full_name"], "kernels.chamfer");
    assert_eq!(json[1]["name"], "delta");
    assert_eq!(json[1]["full_name"], "kernels.delta");
    assert!(json
        .as_array()
        .unwrap()
        .iter()
        .any(|kernel| kernel["full_name"] == "kernels.fauxplugin"));
}

#[test]
fn fauxplugin_kernel_matches_existing_plugin_output() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("fauxplugin")
        .arg("7")
        .output()
        .unwrap();

    assert!(result.status.success());
    assert_eq!(
        String::from_utf8_lossy(&result.stdout),
        "FauxPluginKernel running.\n"
    );
}

#[test]
fn stage_options_reports_rust_metadata() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--options")
        .arg("filters.decimation")
        .output()
        .unwrap();

    assert!(result.status.success());
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("filters.decimation"));
    assert!(stdout.contains("step"));
    assert!(stdout.contains("Keep every Nth point."));
}

#[test]
fn stage_options_supports_json() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("--options")
        .arg("writers.ply")
        .output()
        .unwrap();

    assert!(result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(json
        .as_array()
        .unwrap()
        .iter()
        .any(|option| option["arg"] == "faces"));
}

#[test]
fn stage_options_reports_scoped_ept_options() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("--options")
        .arg("readers.ept")
        .output()
        .unwrap();

    assert!(result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let args: Vec<_> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["arg"].as_str().unwrap())
        .collect();
    assert!(args.contains(&"filename"));
    assert!(args.contains(&"bounds"));
    assert!(args.contains(&"resolution"));
    assert!(args.contains(&"origin"));
    assert!(args.contains(&"ignore_unreadable"));
}

#[test]
fn stage_options_reports_scoped_gdal_reader_options() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("--options")
        .arg("readers.gdal")
        .output()
        .unwrap();

    assert!(result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let args: Vec<_> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["arg"].as_str().unwrap())
        .collect();
    assert!(args.contains(&"filename"));
    assert!(args.contains(&"header"));
    assert!(args.contains(&"gdalopts"));
}

#[test]
fn stage_options_reports_scoped_gdal_writer_options() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("--options")
        .arg("writers.gdal")
        .output()
        .unwrap();

    assert!(result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let args: Vec<_> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["arg"].as_str().unwrap())
        .collect();
    assert!(args.contains(&"filename"));
    assert!(args.contains(&"data_type"));
    assert!(args.contains(&"bounds"));
    assert!(args.contains(&"override_srs"));
    assert!(args.contains(&"default_srs"));
    assert!(args.contains(&"metadata"));
}

#[test]
fn stage_options_reports_scoped_text_reader_options() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("--options")
        .arg("readers.text")
        .output()
        .unwrap();

    assert!(result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let args: Vec<_> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["arg"].as_str().unwrap())
        .collect();
    assert!(args.contains(&"filename"));
    assert!(args.contains(&"separator"));
    assert!(args.contains(&"header"));
    assert!(args.contains(&"skip"));
}

#[test]
fn stage_options_reports_scoped_hexbin_options() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("--options")
        .arg("filters.hexbin")
        .output()
        .unwrap();

    assert!(result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let args: Vec<_> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["arg"].as_str().unwrap())
        .collect();
    assert!(args.contains(&"sample_size"));
    assert!(args.contains(&"threshold"));
    assert!(args.contains(&"edge_size"));
    assert!(args.contains(&"edge_length"));
    assert!(args.contains(&"density"));
}

#[test]
fn stage_options_reports_scoped_smrf_options() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("--options")
        .arg("filters.smrf")
        .output()
        .unwrap();

    assert!(result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let args: Vec<_> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["arg"].as_str().unwrap())
        .collect();
    assert!(args.contains(&"cell"));
    assert!(args.contains(&"slope"));
    assert!(args.contains(&"scalar"));
    assert!(args.contains(&"threshold"));
    assert!(args.contains(&"window"));
    assert!(args.contains(&"returns"));
    assert!(args.contains(&"ground_class"));
    assert!(args.contains(&"other_class"));
    assert!(args.contains(&"only_ground"));
}

#[test]
fn stage_options_reports_scoped_pcd_writer_options() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("--options")
        .arg("writers.pcd")
        .output()
        .unwrap();

    assert!(result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let args: Vec<_> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["arg"].as_str().unwrap())
        .collect();
    assert!(args.contains(&"compression"));
    assert!(args.contains(&"keep_unspecified"));
}

#[test]
fn stage_options_do_not_leak_las_writer_options_to_other_writers() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("--options")
        .arg("writers.bpf")
        .output()
        .unwrap();

    assert!(result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let args: Vec<_> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["arg"].as_str().unwrap())
        .collect();
    assert!(args.contains(&"compression"));
    assert!(args.contains(&"bundledfile"));
    assert!(!args.contains(&"point_format"));
}
