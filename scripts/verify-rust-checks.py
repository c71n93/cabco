"""Exercise the CI commands with disposable copies and deliberate violations."""

from pathlib import Path
import shutil
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[1]
FORMAT = ["cargo", "+stable", "fmt", "--all", "--", "--check"]
CLIPPY = [
    "cargo", "+stable", "clippy", "--workspace", "--all-targets",
    "--all-features", "--", "-D", "warnings", "-D", "missing_docs",
]


def check(directory, command, passes, diagnostic=""):
    result = subprocess.run(command, cwd=directory, capture_output=True, text=True)
    output = result.stdout + result.stderr
    if (result.returncode == 0) != passes or diagnostic not in output:
        raise RuntimeError(f"Unexpected result for {' '.join(command)}:\n{output}")
    print(f"{'PASS' if passes else 'EXPECTED FAILURE'}: {' '.join(command)}")
    if diagnostic:
        print(f"  Confirmed diagnostic: {diagnostic}")


def main():
    with tempfile.TemporaryDirectory(prefix="cabco-rust-checks-") as temporary:
        directory = Path(temporary)
        for name in ["Cargo.toml", "Cargo.lock"]:
            shutil.copy2(ROOT / name, directory / name)
        for name in ["src", "tests"]:
            if (ROOT / name).is_dir():
                shutil.copytree(ROOT / name, directory / name)
        check(directory, FORMAT, passes=True)
        check(directory, CLIPPY, passes=True)

        source = directory / "src" / "lib.rs"
        if not source.exists():
            source.write_text("//! Disposable library for lint probes.\n")
        original = source.read_text()
        source.write_text(original + '\npub fn formatting_probe( )->usize{1}\n')
        malformed = source.read_bytes()
        check(directory, FORMAT, passes=False, diagnostic="Diff in")
        if source.read_bytes() != malformed:
            raise RuntimeError("Formatting check rewrote its input")

        source.write_text(original + '''
/// Disposable probe for a default Clippy warning.
pub fn clippy_probe() -> String {
    let mut text = String::new();
    text.push_str("x");
    text
}
''')
        check(directory, FORMAT, passes=True)
        check(directory, CLIPPY, passes=False, diagnostic="clippy::single_char_add_str")

        source.write_text(original + '\npub fn documentation_probe() {}\n')
        check(directory, CLIPPY, passes=False, diagnostic="missing documentation for a function")
        source.write_text(original)
        check(directory, CLIPPY, passes=True)


if __name__ == "__main__":
    main()
