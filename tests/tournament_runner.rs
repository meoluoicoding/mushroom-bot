use std::process::Command;

#[test]
fn tournament_runner_smoke_test() {
    let exe = env!("CARGO_BIN_EXE_tournament");
    let output = Command::new(exe)
        .args(["--games", "1", "--depth-a", "1", "--depth-b", "1", "--budget", "1", "--seed", "7"])
        .output()
        .expect("failed to run tournament binary");

    assert!(output.status.success(), "tournament runner exited with failure");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let fields: Vec<&str> = stdout.split_whitespace().collect();
    assert!(fields.len() >= 4, "unexpected tournament output: {stdout}");
    assert!(fields[0].parse::<u32>().is_ok(), "win count should be numeric");
    assert!(fields[1].parse::<u32>().is_ok(), "draw count should be numeric");
    assert!(fields[2].parse::<u32>().is_ok(), "loss count should be numeric");
    assert!(fields[3].parse::<f64>().is_ok(), "average margin should be numeric");
}
