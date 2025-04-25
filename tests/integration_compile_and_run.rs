use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

/// Structure to hold the result of running the compiled program
pub struct RunResult {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Compiles, links, and runs an AIC program. Returns exit code, stdout, and stderr.
fn compile_and_run_aic<P: AsRef<Path>>(aic_path: P) -> RunResult {
    let aic_path = aic_path.as_ref();
    let stem = aic_path.file_stem().unwrap().to_str().unwrap();
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let obj_file = temp_dir.path().join(format!("{}.test.o", stem));
    let exe_file = temp_dir.path().join(format!("{}.test.out", stem));

    // Compile to object file (suppress output unless error)
    let status = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "--input",
            aic_path.to_str().unwrap(),
            "-o",
            obj_file.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::null())
        .status()
        .expect("Failed to run cargo build");
    assert!(status.success(), "cargo build failed");

    // Link to executable using mold as the linker (suppress output unless error)
    let status = Command::new("clang")
        .args([
            "-fuse-ld=mold",
            obj_file.to_str().unwrap(),
            "-o",
            exe_file.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::null())
        .status()
        .expect("Failed to run clang with mold");
    assert!(status.success(), "clang (mold) failed");

    // Run and capture output
    let output = Command::new(exe_file.to_str().unwrap())
        .output()
        .expect("Failed to run executable");
    RunResult {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

#[test]
fn test_simple_aic() {
    let result = compile_and_run_aic("tests/fixtures/simple.aic");
    assert_eq!(
        result.code, 84,
        "exit code was {}, expected 84",
        result.code
    );
}

#[test]
fn test_zero_aic() {
    let result = compile_and_run_aic("tests/fixtures/zero.aic");
    assert_eq!(result.code, 0, "exit code was {}, expected 0", result.code);
}

#[test]
fn test_negative_aic() {
    let result = compile_and_run_aic("tests/fixtures/negative.aic");
    // On most platforms, returning a negative value from main results in exit code 1
    assert_eq!(result.code, 1, "exit code was {}, expected 1", result.code);
}

#[test]
fn test_function_call_aic() {
    let actual = compile_and_run_aic("tests/fixtures/function_call.aic").code;
    let expected = 0;
    assert_eq!(
        actual, expected,
        "exit code was {actual}, expected {expected}",
    );
}

#[test]
fn test_let_and_var_aic() {
    let actual = compile_and_run_aic("tests/fixtures/let_and_var.aic").code;
    let expected = 30;
    assert_eq!(
        actual, expected,
        "exit code was {actual}, expected {expected}",
    );
}

#[test]
fn test_if_statement_aic() {
    let actual = compile_and_run_aic("tests/fixtures/if_statement.aic").code;
    let expected = 1;
    assert_eq!(
        actual, expected,
        "exit code was {actual}, expected {expected}",
    );
}

#[test]
fn test_if_statement_else_aic() {
    let actual = compile_and_run_aic("tests/fixtures/if_statement_else.aic").code;
    let expected = 1;
    assert_eq!(
        actual, expected,
        "exit code was {actual}, expected {expected}",
    );
}

#[test]
fn test_if_statement_nested_aic() {
    let actual = compile_and_run_aic("tests/fixtures/if_statement_nested.aic").code;
    let expected = 1;
    assert_eq!(
        actual, expected,
        "exit code was {actual}, expected {expected}",
    );
}

#[test]
fn test_if_statement_else_if_chain_aic() {
    let actual = compile_and_run_aic("tests/fixtures/if_statement_else_if_chain.aic").code;
    let expected = 2;
    assert_eq!(
        actual, expected,
        "exit code was {actual}, expected {expected}",
    );
}

#[test]
fn test_if_statement_else_if_no_else_aic() {
    let actual = compile_and_run_aic("tests/fixtures/if_statement_else_if_no_else.aic").code;
    let expected = 3;
    assert_eq!(
        actual, expected,
        "exit code was {actual}, expected {expected}",
    );
}

#[test]
fn test_boolean_and_comparison_aic() {
    let actual = compile_and_run_aic("tests/fixtures/boolean_and_comparison.aic").code;
    let expected = 42;
    assert_eq!(
        actual, expected,
        "exit code was {actual}, expected {expected}",
    );
}

#[test]
fn test_comments_aic() {
    let actual = compile_and_run_aic("tests/fixtures/comments.aic").code;
    let expected = 30;
    assert_eq!(
        actual, expected,
        "exit code was {actual}, expected {expected}",
    );
}

#[test]
fn test_mutable_var_aic() {
    let actual = compile_and_run_aic("tests/fixtures/mutable_var.aic").code;
    let expected = 15;
    assert_eq!(
        actual, expected,
        "exit code was {actual}, expected {expected}",
    );
}
