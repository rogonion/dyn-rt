//! Ports: roles the core needs from the world. Definitions live HERE;
//! implementations live outside the core, in adapter crates
//! (`dyn-rt-json-infra`).

use crate::path::SliceSelector;

/// Tokenizer/splitter port used by the path parse use cases.
///
/// Each method is one explicit step of the parse pipeline; implementations
/// (e.g. the regex tokenizer in `dyn-rt-json-infra`) provide the actual
/// matching mechanics, named per step.
pub trait PathTokenizer {
    /// Step 1: split a path on the recursive descent operator `..`,
    /// respecting brackets and quotes (a `..` inside `'...'`/`[...]` is not
    /// a delimiter).
    fn split_recursive_descent<'a>(&self, path: &'a str) -> Vec<&'a str>;

    /// Step 2: split a descent part on the dot `.` delimiter, respecting
    /// brackets and quotes.
    fn split_dot_notation<'a>(&self, part: &'a str) -> Vec<&'a str>;

    /// Step 3: split a dot part into bracket contents (the inner text of
    /// each `[...]`) and non-bracket text, e.g. `book[0][3]title` ->
    /// `"book"`, `"0"`, `"3"`, `"title"`.
    fn split_brackets<'a>(&self, part: &'a str) -> Vec<&'a str>;

    /// Step 4a: split a bracket content on union commas `,`, respecting
    /// quotes.
    fn split_union<'a>(&self, content: &'a str) -> Vec<&'a str>;

    /// Step 4b: if the token is a quoted string (`'...'`, `"..."`, `` `...` ``),
    /// return the unquoted content.
    fn extract_quoted(&self, token: &str) -> Option<String>;

    /// Step 4b: if the token is a positional index (`-?digits`), return it.
    fn parse_index(&self, token: &str) -> Option<isize>;

    /// Step 4b: if the token is a slice (`start:end:step`, any part optional),
    /// return it; `None` when the token is not a slice.
    fn parse_slice(&self, token: &str) -> Option<SliceSelector>;
}
