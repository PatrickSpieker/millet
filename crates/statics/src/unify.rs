use crate::error::ErrorKind;
use crate::st::St;
use crate::types::{MetaTyVar, Overload, Subst, SubstEntry, Ty, TyVarKind};
use crate::util::apply;

pub(crate) fn unify<I>(st: &mut St, want: Ty, got: Ty, idx: I)
where
  I: Into<hir::Idx>,
{
  let e = match unify_(st.subst(), want.clone(), got.clone()) {
    Ok(()) => return,
    Err(e) => e,
  };
  let e = match e {
    UnifyError::OccursCheck(mv, ty) => ErrorKind::Circularity(mv, ty),
    UnifyError::HeadMismatch => ErrorKind::MismatchedTypes(want, got),
    UnifyError::OverloadMismatch(ov) => ErrorKind::OverloadMismatch(ov, want, got),
  };
  st.err(idx, e);
}

#[derive(Debug)]
enum UnifyError {
  OccursCheck(MetaTyVar, Ty),
  HeadMismatch,
  OverloadMismatch(Overload),
}

/// `want` and `got` will have `subst` applied to them upon entry to this function
fn unify_(subst: &mut Subst, mut want: Ty, mut got: Ty) -> Result<(), UnifyError> {
  apply(subst, &mut want);
  apply(subst, &mut got);
  match (want, got) {
    (Ty::None, _) | (_, Ty::None) => Ok(()),
    (Ty::BoundVar(want), Ty::BoundVar(got)) => head_match(want == got),
    (Ty::MetaVar(mv), ty) | (ty, Ty::MetaVar(mv)) => {
      if let Ty::MetaVar(mv2) = &ty {
        if mv == *mv2 {
          return Ok(());
        }
      }
      if occurs(&mv, &ty) {
        return Err(UnifyError::OccursCheck(mv, ty));
      }
      match subst.insert(mv, SubstEntry::Solved(ty.clone())) {
        None | Some(SubstEntry::Kind(TyVarKind::Equality)) => {}
        Some(SubstEntry::Kind(TyVarKind::Overloaded(ov))) => match ty {
          Ty::None => {}
          Ty::Con(args, s) => {
            if ov.to_syms().contains(&s) {
              assert!(args.is_empty())
            } else {
              return Err(UnifyError::OverloadMismatch(ov));
            }
          }
          Ty::MetaVar(mv2) => {
            match subst.insert(mv2, SubstEntry::Kind(TyVarKind::Overloaded(ov))) {
              None => {}
              Some(entry) => match entry {
                SubstEntry::Solved(ty) => unreachable!("meta var already solved to {ty:?}"),
                SubstEntry::Kind(kind) => match kind {
                  // all overload types are equality types
                  TyVarKind::Equality => {}
                  TyVarKind::Overloaded(_) => unreachable!("an overloaded ty var was in scope"),
                },
              },
            }
          }
          Ty::BoundVar(_) | Ty::FixedVar(_) | Ty::Record(_) | Ty::Fn(_, _) => {
            return Err(UnifyError::OverloadMismatch(ov))
          }
        },
        Some(SubstEntry::Solved(ty)) => unreachable!("meta var already solved to {ty:?}"),
      }
      Ok(())
    }
    (Ty::FixedVar(want), Ty::FixedVar(got)) => head_match(want == got),
    (Ty::Record(want_rows), Ty::Record(mut got_rows)) => {
      for (lab, want) in want_rows {
        match got_rows.remove(&lab) {
          None => return Err(UnifyError::HeadMismatch),
          Some(got) => unify_(subst, want, got)?,
        }
      }
      if got_rows.is_empty() {
        Ok(())
      } else {
        Err(UnifyError::HeadMismatch)
      }
    }
    (Ty::Con(want_args, want_sym), Ty::Con(got_args, got_sym)) => {
      head_match(want_sym == got_sym)?;
      assert_eq!(want_args.len(), got_args.len());
      for (want, got) in want_args.into_iter().zip(got_args) {
        unify_(subst, want, got)?;
      }
      Ok(())
    }
    (Ty::Fn(want_param, want_res), Ty::Fn(got_param, got_res)) => {
      unify_(subst, *want_param, *got_param)?;
      unify_(subst, *want_res, *got_res)
    }
    (Ty::BoundVar(_) | Ty::FixedVar(_) | Ty::Record(_) | Ty::Con(_, _) | Ty::Fn(_, _), _) => {
      Err(UnifyError::HeadMismatch)
    }
  }
}

fn head_match(b: bool) -> Result<(), UnifyError> {
  if b {
    Ok(())
  } else {
    Err(UnifyError::HeadMismatch)
  }
}

fn occurs(mv: &MetaTyVar, ty: &Ty) -> bool {
  match ty {
    Ty::None | Ty::BoundVar(_) | Ty::FixedVar(_) => false,
    Ty::MetaVar(mv2) => mv == mv2,
    Ty::Record(rows) => rows.values().any(|t| occurs(mv, t)),
    Ty::Con(args, _) => args.iter().any(|t| occurs(mv, t)),
    Ty::Fn(param, res) => occurs(mv, param) || occurs(mv, res),
  }
}
