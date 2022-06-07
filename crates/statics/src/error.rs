use crate::fmt_util::comma_seq;
use crate::pat_match::{Con, Pat, VariantName};
use crate::types::{MetaTyVar, Overload, Sym, Syms, Ty};
use pattern_match::RawPat;
use std::fmt;

/// A statics error.
#[derive(Debug)]
pub struct Error {
  pub(crate) idx: hir::Idx,
  pub(crate) kind: ErrorKind,
}

impl Error {
  /// Returns the [`hir::Idx`] for this error.
  pub fn idx(&self) -> hir::Idx {
    self.idx
  }

  /// Displays this error.
  pub fn display<'a>(&'a self, syms: &'a Syms) -> impl fmt::Display + 'a {
    ErrorKindDisplay {
      kind: &self.kind,
      syms,
    }
  }
}

#[derive(Debug)]
pub(crate) enum ErrorKind {
  Unsupported(&'static str),
  Undefined(Item, hir::Name),
  Duplicate(Item, hir::Name),
  Missing(Item, hir::Name),
  Extra(Item, hir::Name),
  Circularity(MetaTyVar, Ty),
  MismatchedTypes(Ty, Ty),
  OverloadMismatch(Overload, Ty, Ty),
  AppLhsNotFn(Ty),
  DuplicateLab(hir::Lab),
  RealPat,
  UnreachablePattern,
  NonExhaustiveMatch(Vec<Pat>),
  NonExhaustiveBinding(Vec<Pat>),
  PatValIdStatus,
  ConPatMustNotHaveArg,
  ConPatMustHaveArg,
  InvalidAsPatName(hir::Name),
  TyNameEscape(Sym),
  ValRecExpNotFn,
  WrongNumTyArgs(usize, usize),
  ExnCopyNotExnIdStatus,
  InvalidRebindName(hir::Name),
  PolymorphicExn,
  WrongIdStatus(hir::Name),
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum Item {
  Val,
  Ty,
  TyVar,
  Struct,
  Sig,
  Functor,
}

impl fmt::Display for Item {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Item::Val => f.write_str("value"),
      Item::Ty => f.write_str("type"),
      Item::TyVar => f.write_str("type variable"),
      Item::Struct => f.write_str("structure"),
      Item::Sig => f.write_str("signature"),
      Item::Functor => f.write_str("functor"),
    }
  }
}

struct ErrorKindDisplay<'a> {
  kind: &'a ErrorKind,
  syms: &'a Syms,
}

impl fmt::Display for ErrorKindDisplay<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self.kind {
      ErrorKind::Unsupported(s) => write!(f, "unsupported language construct: {s}"),
      ErrorKind::Undefined(item, name) => write!(f, "undefined {item}: {name}"),
      ErrorKind::Duplicate(item, name) => write!(f, "duplicate {item}: {name}"),
      ErrorKind::Missing(item, name) => write!(f, "missing {item} required by signature: {name}"),
      ErrorKind::Extra(item, name) => write!(f, "extra {item} not present in signature: {name}"),
      ErrorKind::Circularity(_, ty) => {
        write!(f, "circularity: {}", ty.display(self.syms))
      }
      ErrorKind::MismatchedTypes(want, got) => write!(
        f,
        "expected {}, found {}",
        want.display(self.syms),
        got.display(self.syms)
      ),
      ErrorKind::OverloadMismatch(ov, want, got) => write!(
        f,
        "expected {} with {}, found {}",
        want.display(self.syms),
        // TODO make this programmatic?
        match ov {
          Overload::WordInt => "word or int",
          Overload::RealInt => "real or int",
          Overload::Num => "word, real, or int",
          Overload::NumTxt => "word, real, int, string, or char",
        },
        got.display(self.syms)
      ),
      ErrorKind::AppLhsNotFn(got) => write!(
        f,
        "expected a function type, got {}",
        got.display(self.syms)
      ),
      ErrorKind::DuplicateLab(lab) => write!(f, "duplicate label: {lab}"),
      ErrorKind::RealPat => f.write_str("real literal used as a pattern"),
      ErrorKind::UnreachablePattern => f.write_str("unreachable pattern"),
      ErrorKind::NonExhaustiveMatch(pats) => non_exhaustive(f, self.syms, pats, "match"),
      ErrorKind::NonExhaustiveBinding(pats) => non_exhaustive(f, self.syms, pats, "binding"),
      ErrorKind::PatValIdStatus => f.write_str("value binding used as a pattern"),
      ErrorKind::ConPatMustNotHaveArg => f.write_str("unexpected argument for constructor pattern"),
      ErrorKind::ConPatMustHaveArg => f.write_str("missing argument for constructor pattern"),
      ErrorKind::InvalidAsPatName(name) => write!(f, "invalid `as` pat name: {name}"),
      ErrorKind::TyNameEscape(sym) => write!(
        f,
        "type name escapes its scope: {}",
        self.syms.get(sym).unwrap().0
      ),
      ErrorKind::ValRecExpNotFn => f.write_str("the expression for a `val rec` was not a `fn`"),
      ErrorKind::WrongNumTyArgs(want, got) => {
        let s = if *want == 1 { "" } else { "s" };
        write!(f, "expected {want} type argument{s}, found {got}")
      }
      ErrorKind::ExnCopyNotExnIdStatus => f.write_str("not an exception"),
      ErrorKind::InvalidRebindName(name) => write!(f, "cannot re-bind name: {name}"),
      ErrorKind::PolymorphicExn => f.write_str("cannot have a polymorphic `exception`"),
      ErrorKind::WrongIdStatus(name) => write!(f, "incompatible identifier statuses: {name}"),
    }
  }
}

fn non_exhaustive(
  f: &mut fmt::Formatter<'_>,
  syms: &Syms,
  pats: &[Pat],
  kind: &str,
) -> fmt::Result {
  write!(f, "non-exhaustive {kind}: missing ")?;
  assert!(!pats.is_empty());
  let max_len = 2;
  let iter = pats.iter().take(max_len).map(|pat| PatDisplay {
    pat,
    syms,
    prec: PatPrec::Min,
  });
  comma_seq(f, iter)?;
  if pats.len() > max_len {
    write!(f, ", and {} others", pats.len() - max_len)?;
  }
  Ok(())
}

struct PatDisplay<'a> {
  pat: &'a Pat,
  syms: &'a Syms,
  prec: PatPrec,
}

impl<'a> fmt::Display for PatDisplay<'a> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let (con, args) = match &self.pat.raw {
      RawPat::Con(a, b) => (a, b),
      RawPat::Or(_) => unreachable!(),
    };
    match con {
      Con::Any => f.write_str("_")?,
      Con::Int(i) => write!(f, "{i}")?,
      Con::Word(w) => write!(f, "0w{w}")?,
      // TODO maybe not accurate
      Con::Char(c) => write!(f, "#\"{}\"", c.escape_ascii())?,
      Con::String(s) => f.write_str(s.as_str())?,
      Con::Record(labs) => {
        assert_eq!(labs.len(), args.len());
        let is_tuple = labs
          .iter()
          .enumerate()
          .all(|(idx, lab)| hir::Lab::tuple(idx) == *lab);
        if is_tuple {
          f.write_str("(")?;
          comma_seq(
            f,
            args.iter().map(|pat| PatDisplay {
              pat,
              syms: self.syms,
              prec: PatPrec::Min,
            }),
          )?;
          f.write_str(")")?;
        } else {
          f.write_str("{")?;
          comma_seq(
            f,
            labs.iter().zip(args).map(|(lab, pat)| RowDisplay {
              lab,
              pat,
              syms: self.syms,
            }),
          )?;
          f.write_str("}")?;
        }
        return Ok(());
      }
      Con::Variant(_, name) => {
        let needs_paren = !args.is_empty() && matches!(self.prec, PatPrec::App);
        if needs_paren {
          f.write_str("(")?;
        }
        match name {
          VariantName::Name(name) => f.write_str(name.as_str())?,
          VariantName::Exn(exn) => f.write_str(self.syms.get_exn(exn).0.as_str())?,
        }
        if args.is_empty() {
          return Ok(());
        }
        f.write_str(" ")?;
        comma_seq(
          f,
          args.iter().map(|pat| PatDisplay {
            pat,
            syms: self.syms,
            prec: PatPrec::App,
          }),
        )?;
        if needs_paren {
          f.write_str(")")?;
        }
        return Ok(());
      }
    }
    // if got here, this is scon/any
    assert!(args.is_empty());
    Ok(())
  }
}

enum PatPrec {
  Min,
  App,
}

struct RowDisplay<'a> {
  lab: &'a hir::Lab,
  pat: &'a Pat,
  syms: &'a Syms,
}

impl<'a> fmt::Display for RowDisplay<'a> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.lab.fmt(f)?;
    f.write_str(": ")?;
    let pd = PatDisplay {
      pat: self.pat,
      syms: self.syms,
      prec: PatPrec::Min,
    };
    pd.fmt(f)
  }
}
