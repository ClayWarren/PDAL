use std::path::{Path, PathBuf};
use std::process::Command;

fn data_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn run_eval(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("eval")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn eval_scores_a_classified_file_against_itself() {
    let input = data_path("test/data/las/interesting.las");
    let result = run_eval(&[
        input.to_str().unwrap(),
        input.to_str().unwrap(),
        "--labels=1,2",
    ]);
    assert!(
        result.status.success(),
        "pdal-rs eval failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    // Comparing a file with itself yields a perfect classification.
    assert_eq!(report["overall_accuracy"], 1.0);
    assert_eq!(report["mean_intersection_over_union"], 1.0);
    assert_eq!(report["f1_score"], 1.0);

    let labels = report["labels"].as_array().unwrap();
    assert_eq!(labels.len(), 2);
    // interesting.las carries 789 class-1 points and 276 class-2 points.
    assert_eq!(labels[0]["label"], 1);
    assert_eq!(labels[0]["support"], 789);
    assert_eq!(labels[1]["label"], 2);
    assert_eq!(labels[1]["support"], 276);
    for label in labels {
        assert_eq!(label["intersection_over_union"], 1.0);
        assert_eq!(label["precision"], 1.0);
        assert_eq!(label["sensitivity"], 1.0);
    }

    // A perfect prediction has an all-diagonal confusion matrix.
    assert_eq!(
        report["confusion_matrix"],
        serde_json::json!([[789, 0, 0], [0, 276, 0], [0, 0, 0]])
    );
}

#[test]
fn eval_supports_named_paths_and_separated_options() {
    let input = data_path("test/data/las/interesting.las");
    let result = run_eval(&[
        "--predicted",
        input.to_str().unwrap(),
        "--truth",
        input.to_str().unwrap(),
        "--labels",
        "1,2",
        "--prediction_dim",
        "Classification",
        "--truth_dim",
        "Classification",
    ]);
    assert!(
        result.status.success(),
        "pdal-rs eval failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["overall_accuracy"], 1.0);
    assert_eq!(report["confusion_matrix"][0][0], 789);
}

#[test]
fn eval_without_labels_fails() {
    let input = data_path("test/data/las/interesting.las");
    let result = run_eval(&[input.to_str().unwrap(), input.to_str().unwrap()]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("--labels"));
}

#[test]
fn eval_without_paths_prints_usage_and_fails() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("eval")
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("pdal eval <predicted> <truth>"));
}

#[test]
fn eval_rejects_a_missing_dimension() {
    let input = data_path("test/data/las/interesting.las");
    let result = run_eval(&[
        input.to_str().unwrap(),
        input.to_str().unwrap(),
        "--labels=1,2",
        "--truth_dim=NoSuchDimension",
    ]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("NoSuchDimension"));
}

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_eval_matches_rust_eval() {
    let input = data_path("test/data/las/interesting.las");

    let installed = Command::new("pdal")
        .arg("eval")
        .arg(format!("--predicted={}", input.display()))
        .arg(format!("--truth={}", input.display()))
        .arg("--labels=1,2")
        .output()
        .expect("failed to execute installed pdal");
    assert!(
        installed.status.success(),
        "installed pdal eval failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&installed.stdout),
        String::from_utf8_lossy(&installed.stderr)
    );

    let rust = run_eval(&[
        input.to_str().unwrap(),
        input.to_str().unwrap(),
        "--labels=1,2",
    ]);
    assert!(rust.status.success());

    let installed_json: serde_json::Value = serde_json::from_slice(&installed.stdout).unwrap();
    let rust_json: serde_json::Value = serde_json::from_slice(&rust.stdout).unwrap();

    // PDAL emits the confusion matrix as a (still valid JSON) string.
    let installed_matrix: serde_json::Value =
        serde_json::from_str(installed_json["confusion_matrix"].as_str().unwrap()).unwrap();
    assert_eq!(rust_json["confusion_matrix"], installed_matrix);

    for key in [
        "overall_accuracy",
        "mean_intersection_over_union",
        "f1_score",
    ] {
        assert_eq!(
            rust_json[key].as_f64().unwrap(),
            installed_json[key].as_f64().unwrap(),
            "metric '{key}' differs"
        );
    }

    let installed_labels = installed_json["labels"].as_array().unwrap();
    let rust_labels = rust_json["labels"].as_array().unwrap();
    assert_eq!(rust_labels.len(), installed_labels.len());
    for (rust_label, installed_label) in rust_labels.iter().zip(installed_labels) {
        assert_eq!(
            rust_label["support"].as_u64(),
            installed_label["support"].as_u64()
        );
    }
}
