//! Parsing patterns.

use crate::parser::{Entered, ErrorKind, Exited, Expected, Infix, Parser};
use crate::ty::{ty, ty_annotation};
use crate::util::{
  comma_sep, eat_name_star, lab, must, name_star, path, path_infix, path_no_infix, scon,
  should_break, InfixErr,
};
use sml_syntax::SyntaxKind as SK;

pub(crate) fn pat(p: &mut Parser<'_>, infix: InfixErr) -> Option<Exited> {
  pat_prec(p, PatPrec::Min, infix)
}

enum ConPatState {
  /// we got the head of a con pat, but we're looking for the arg.
  Entered(Entered),
  /// we know we either got the arg or didn't. so we're not parsing a con pat right now.
  Exited(Exited),
}

impl ConPatState {
  fn exit(self, p: &mut Parser<'_>) -> Exited {
    match self {
      ConPatState::Entered(en) => p.exit(en, SK::ConPat),
      ConPatState::Exited(ex) => ex,
    }
  }
}

enum AtPatHd {
  /// corresponds to `ConPatState::Entered`.
  ConPatArg(Entered),
  /// we're parsing an infix pat.
  Infix(ConPatState, Infix),
}

/// kind of gross for the tricky ones.
fn pat_prec(p: &mut Parser<'_>, min_prec: PatPrec, infix: InfixErr) -> Option<Exited> {
  // con pat with arg, or infix pat
  let mut state = if name_star(p, 0) || (p.at(SK::OpKw) && name_star(p, 1)) {
    let en = p.enter();
    if p.at(SK::OpKw) {
      path_infix(p);
    } else {
      match infix {
        InfixErr::No => {
          path(p);
        }
        InfixErr::Yes => path_no_infix(p),
      }
    }
    ConPatState::Entered(en)
  } else {
    ConPatState::Exited(at_pat(p, infix)?)
  };
  loop {
    let ex = if at_pat_hd(p) {
      let at_pat_hd = if name_star(p, 0) {
        let tok = p.peek().unwrap();
        match p.get_infix(tok.text) {
          None => match state {
            ConPatState::Entered(en) => AtPatHd::ConPatArg(en),
            ConPatState::Exited(_) => {
              p.error(ErrorKind::NotInfix);
              AtPatHd::Infix(state, Infix::left(0))
            }
          },
          Some(op_info) => AtPatHd::Infix(state, op_info),
        }
      } else {
        match state {
          ConPatState::Entered(en) => AtPatHd::ConPatArg(en),
          ConPatState::Exited(_) => break,
        }
      };
      match at_pat_hd {
        AtPatHd::ConPatArg(en) => {
          must(p, |p| at_pat(p, infix), Expected::Pat);
          p.exit(en, SK::ConPat)
        }
        AtPatHd::Infix(st, op_info) => {
          state = st;
          match min_prec {
            PatPrec::Min | PatPrec::Or => {}
            PatPrec::Infix(min_prec) => {
              if should_break(p, op_info, min_prec) {
                break;
              }
            }
          }
          let ex = state.exit(p);
          let en = p.precede(ex);
          p.bump();
          must(p, |p| pat_prec(p, PatPrec::Infix(op_info), infix), Expected::Pat);
          p.exit(en, SK::InfixPat)
        }
      }
    } else if p.at(SK::Bar) {
      match min_prec {
        PatPrec::Min => {}
        PatPrec::Or | PatPrec::Infix(_) => break,
      }
      let ex = state.exit(p);
      let en = p.precede(ex);
      p.bump();
      must(p, |p| pat_prec(p, PatPrec::Or, infix), Expected::Pat);
      p.exit(en, SK::OrPat)
    } else if p.at(SK::Colon) {
      match min_prec {
        PatPrec::Min | PatPrec::Or => {}
        PatPrec::Infix(_) => break,
      }
      let ex = state.exit(p);
      let en = p.precede(ex);
      p.bump();
      ty(p);
      p.exit(en, SK::TypedPat)
    } else if p.at(SK::AsKw) {
      match min_prec {
        PatPrec::Min | PatPrec::Or => {}
        PatPrec::Infix(_) => break,
      }
      let ex = state.exit(p);
      let en = p.precede(ex);
      must(p, |p| as_pat_tl(p, infix), Expected::Pat);
      p.exit(en, SK::AsPat)
    } else {
      break;
    };
    state = ConPatState::Exited(ex);
  }
  Some(state.exit(p))
}

#[derive(Debug, Clone, Copy)]
enum PatPrec {
  Min,
  Or,
  Infix(Infix),
}

/// when adding more cases to this, update [`at_pat_hd`].
pub(crate) fn at_pat(p: &mut Parser<'_>, infix: InfixErr) -> Option<Exited> {
  let en = p.enter();
  let ex = if scon(p) {
    p.bump();
    p.exit(en, SK::SConPat)
  } else if p.at(SK::Underscore) {
    p.bump();
    p.exit(en, SK::WildcardPat)
  } else if p.at(SK::OpKw) {
    path_infix(p);
    p.exit(en, SK::ConPat)
  } else if name_star(p, 0) {
    match infix {
      InfixErr::No => {
        path(p);
      }
      InfixErr::Yes => path_no_infix(p),
    }
    p.exit(en, SK::ConPat)
  } else if p.at(SK::LCurly) {
    p.bump();
    comma_sep(p, SK::RCurly, SK::PatRow, |p| {
      let en = p.enter();
      if p.at(SK::DotDotDot) {
        p.bump();
        p.exit(en, SK::RestPatRow);
      } else if p.at_n(1, SK::Eq) {
        lab(p);
        p.eat(SK::Eq);
        must(p, |p| pat(p, infix), Expected::Pat);
        p.exit(en, SK::LabAndPatPatRow);
      } else {
        eat_name_star(p);
        let _ = ty_annotation(p);
        let _ = as_pat_tl(p, infix);
        p.exit(en, SK::LabPatRow);
      }
    });
    p.exit(en, SK::RecordPat)
  } else if p.at(SK::LRound) {
    p.bump();
    let kind = at_pat_l_round(p, infix);
    p.exit(en, kind)
  } else if p.at(SK::LSquare) {
    p.bump();
    comma_sep(p, SK::RSquare, SK::PatArg, |p| {
      must(p, |p| pat(p, infix), Expected::Pat);
    });
    p.exit(en, SK::ListPat)
  } else if p.at(SK::Hash) {
    p.bump();
    let list = p.enter();
    p.eat(SK::LSquare);
    comma_sep(p, SK::RSquare, SK::PatArg, |p| {
      must(p, |p| pat(p, infix), Expected::Pat);
    });
    p.exit(list, SK::ListPat);
    p.exit(en, SK::VectorPat)
  } else {
    p.abandon(en);
    return None;
  };
  Some(ex)
}

/// see [`at_pat`].
fn at_pat_hd(p: &mut Parser<'_>) -> bool {
  scon(p)
    || p.at(SK::Underscore)
    || p.at(SK::OpKw)
    || name_star(p, 0)
    || p.at(SK::LCurly)
    || p.at(SK::LRound)
    || p.at(SK::LSquare)
    || p.at(SK::Hash)
}

fn at_pat_l_round(p: &mut Parser<'_>, infix: InfixErr) -> SK {
  if p.at(SK::RRound) {
    p.bump();
    return SK::TuplePat;
  }
  let en = p.enter();
  must(p, |p| pat(p, infix), Expected::Pat);
  if p.at(SK::RRound) {
    p.abandon(en);
    p.bump();
    return SK::ParenPat;
  }
  p.eat(SK::Comma);
  p.exit(en, SK::PatArg);
  loop {
    let en = p.enter();
    must(p, |p| pat(p, infix), Expected::Pat);
    if p.at(SK::Comma) {
      p.bump();
      p.exit(en, SK::PatArg);
    } else {
      p.exit(en, SK::PatArg);
      p.eat(SK::RRound);
      break;
    }
  }
  SK::TuplePat
}

fn as_pat_tl(p: &mut Parser<'_>, infix: InfixErr) -> Option<Exited> {
  if p.at(SK::AsKw) {
    let en = p.enter();
    p.bump();
    must(p, |p| pat(p, infix), Expected::Pat);
    Some(p.exit(en, SK::AsPatTail))
  } else {
    None
  }
}
