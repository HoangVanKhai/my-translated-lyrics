use std::process::Command;

use command_extra::CommandExtra;

const LYRICS_AI_INSTRUCTIONS: &str = env!("CARGO_BIN_EXE_lyrics-ai-instructions");

#[test]
fn ai_instructions_up_to_date() {
    let output = Command::new(LYRICS_AI_INSTRUCTIONS)
        .with_arg(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("spawn lyrics-ai-instructions");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stdout = stdout.trim();
    if !stdout.is_empty() {
        eprintln!("STDOUT:\n{stdout}\n");
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = stderr.trim();
    if !stderr.is_empty() {
        eprintln!("STDERR:\n{stderr}\n");
    }
    assert!(
        output.status.success(),
        "AI instruction files are outdated. Run `cargo run --bin lyrics-ai-instructions -- --generate .` to update.",
    );
}
