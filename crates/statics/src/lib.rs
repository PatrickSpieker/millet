//! Static analysis.
//!
//! With help from [this article][1].
//!
//! [1]: http://dev.stephendiehl.com/fun/006_hindley_milner.html

#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(rust_2018_idioms)]

mod dec;
mod error;
mod exp;
mod fmt_util;
mod pat;
mod pat_match;
mod st;
mod std_basis;
mod top_dec;
mod ty;
mod types;
mod unify;
mod util;

pub use error::Error;
pub use st::Statics;
pub use types::Syms;

/// Does the checks.
pub fn get(statics: &mut Statics, arenas: &hir::Arenas, top_decs: &[hir::TopDecIdx]) {
  let mut st = st::St::new(std::mem::take(&mut statics.syms));
  for &top_dec in top_decs {
    top_dec::get(&mut st, &mut statics.bs, arenas, top_dec);
  }
  let (syms, errors) = st.finish();
  statics.syms = syms;
  statics.errors.extend(errors);
}
