//! Static analysis.
//!
//! With help from [this article][1].
//!
//! [1]: http://dev.stephendiehl.com/fun/006_hindley_milner.html

#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(rust_2018_idioms)]
#![allow(dead_code)]

mod dec;
mod error;
mod exp;
mod pat;
mod pat_match;
mod st;
mod top_dec;
mod ty;
mod types;
mod unify;
mod util;

pub use error::Error;
pub use st::St;

/// Does the checks.
pub fn get(arenas: &hir::Arenas, top_decs: &[hir::TopDec]) -> Vec<Error> {
  let cx = types::Cx::default();
  let mut st = st::St::default();
  for top_dec in top_decs {
    top_dec::get(&mut st, &cx, arenas, top_dec);
  }
  st.finish()
}
