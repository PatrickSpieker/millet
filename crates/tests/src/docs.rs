//! Tests to make sure Millet behaves as expected on the public documentation.

use crate::check::{go, Outcome};
use diagnostic_util::Severity;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag};

const SML: &str = "sml";
const MINI_STD_BASIS: &str = include_str!("mini-std-basis.sml");

fn check_all(contents: &str) {
  let mut options = Options::empty();
  options.insert(Options::ENABLE_TABLES);
  let parser = Parser::new_ext(contents, options);
  let mut inside = false;
  let mut ignore_next = false;
  let mut ac = String::new();
  for ev in parser {
    match ev {
      Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
        if lang.as_ref() == SML {
          inside = true;
        }
      }
      Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
        if lang.as_ref() == SML {
          if !ignore_next {
            let prog = &[MINI_STD_BASIS, ac.as_str()];
            go(prog, analysis::StdBasis::Minimal, Outcome::Pass, Severity::Warning);
          }
          ac.clear();
          inside = false;
          ignore_next = false;
        }
      }
      Event::Text(s) => {
        if inside {
          ac.push_str(s.as_ref());
        }
      }
      Event::Html(s) => {
        if s.trim_start().starts_with("<!-- @ignore ") {
          ignore_next = true;
        }
      }
      _ => {}
    }
  }
}

#[test]
fn diagnostics() {
  check_all(include_str!("../../../docs/diagnostics.md"));
}

#[test]
fn tokens() {
  check_all(include_str!("../../../docs/tokens.md"));
}
