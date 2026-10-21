"""Exercise the checker CLI with real compiler-produced API data."""

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


CHECKER = Path(__file__).with_name("check.py")
TOOLCHAIN = "+nightly-2025-07-30"


class RustdocSectionsTest(unittest.TestCase):
    def check_source(self, source):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source_path = root / "fixture.rs"
            source_path.write_text(source)
            generated = subprocess.run(
                ["rustdoc", TOOLCHAIN, str(source_path), "--edition=2024",
                 "--crate-type=lib", "-Z", "unstable-options", "--output-format=json",
                 "--document-private-items", "--document-hidden-items", "-o", str(root)],
                capture_output=True, text=True,
            )
            self.assertEqual(generated.returncode, 0, generated.stderr)
            return subprocess.run(
                [sys.executable, str(CHECKER), str(root / "fixture.json")],
                capture_output=True, text=True,
            )

    def test_indented_code_does_not_hide_real_sections(self):
        result = self.check_source('''
/// Example of literal Markdown:
///
///     ```
///     some code
///
/// # Returns
/// The answer, always 42.
pub fn answer() -> u8 { 42 }
''')
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_public_contracts_and_unit_exemptions_pass(self):
        result = self.check_source('''
pub fn no_parameters() {}
pub fn explicit_unit() -> () {}
pub struct Api;
impl Api {
    pub fn receiver_only(&self) {}
    /// # Arguments
    /// - `value`: A value in the full u8 range.
    /// # Returns
    /// The supplied value, unchanged, as Ok.
    /// # Errors
    /// Never fails.
    pub async fn echo(&self, value: u8) -> Result<u8, ()> { Ok(value) }
}
pub trait Contract {
    /// # Arguments
    /// - `value` - A value in the full u8 range.
    /// # Returns
    /// The supplied value, unchanged.
    fn echo(&self, value: u8) -> u8;
}
impl Contract for Api {
    fn echo(&self, value: u8) -> u8 { value }
}
mod hidden {
    pub fn inaccessible(value: u8) -> u8 { value }
}
pub(crate) fn internal(value: u8) -> u8 { value }
''')
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_missing_sections_and_parameter_entries_fail(self):
        cases = [
            ("", "missing # Arguments"),
            ("/// # Arguments\n/// - `value`: Input.\n", "missing # Returns"),
            ("/// # Arguments\n/// - `other`: Input.\n/// # Returns\n/// Input.\n",
             "undocumented parameter `value`"),
            ("/// # Arguments\n/// - `value`:\n/// # Returns\n/// Input.\n",
             "undocumented parameter `value`"),
            ("/// # Arguments\n/// - `value`: Input.\n/// # Returns\n",
             "missing # Returns"),
            ("/// ```rust\n/// # Arguments\n/// - `value`: Input.\n"
             "/// # Returns\n/// Input.\n/// ```\n", "missing # Arguments"),
        ]
        for docs, diagnostic in cases:
            with self.subTest(diagnostic=diagnostic, docs=docs):
                result = self.check_source(docs + "pub fn echo(value: u8) -> u8 { value }")
                self.assertEqual(result.returncode, 1)
                self.assertIn(diagnostic, result.stderr)
                self.assertIn("fixture.rs:", result.stderr)

    def test_every_explicit_parameter_needs_its_own_entry(self):
        result = self.check_source('''
/// # Arguments
/// - `first`: First value; second is also mentioned here.
pub fn consume(first: u8, second: u8) {}
''')
        self.assertEqual(result.returncode, 1)
        self.assertIn("undocumented parameter `second`", result.stderr)

    def test_public_methods_and_reexports_are_checked(self):
        for exposure in [
            "pub use hidden::Api;",
            "pub type Api = hidden::Api;",
            "pub use hidden::*;",
            "pub struct Wrapper { pub api: hidden::Api }",
            "pub enum Wrapper { Api(hidden::Api) }",
            "pub fn expose() -> hidden::Api { hidden::Api }",
        ]:
            with self.subTest(exposure=exposure):
                result = self.check_source('''
mod hidden {
    pub struct Api;
    impl Api {
        pub fn undocumented(&self, value: u8) -> u8 { value }
    }
}
''' + exposure)
                self.assertEqual(result.returncode, 1)
                self.assertIn("undocumented: missing # Arguments", result.stderr)
                self.assertIn("undocumented: missing # Returns", result.stderr)

    def test_trait_declarations_and_macro_functions_are_checked(self):
        for source in [
            "pub trait Api { fn undocumented(&self, value: u8) -> u8; }",
            "macro_rules! api { () => { pub fn undocumented(value: u8) -> u8 { value } } } api!();",
            "#[doc(hidden)] pub fn undocumented(value: u8) -> u8 { value }",
        ]:
            with self.subTest(source=source):
                result = self.check_source(source)
                self.assertEqual(result.returncode, 1)
                self.assertIn("missing # Arguments", result.stderr)
                self.assertIn("missing # Returns", result.stderr)

    def test_reexported_free_function_is_checked(self):
        result = self.check_source('''
mod hidden { pub fn undocumented(value: u8) -> u8 { value } }
pub use hidden::undocumented;
''')
        self.assertEqual(result.returncode, 1)
        self.assertIn("undocumented: missing # Arguments", result.stderr)



if __name__ == "__main__":
    unittest.main()
