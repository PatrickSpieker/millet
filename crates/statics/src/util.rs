use crate::error::{ErrorKind, Item};
use crate::st::St;
use crate::types::{Env, RecordTy, Subst, SubstEntry, Ty, TyInfo, TyScheme};
use fast_hash::FxHashMap;

pub(crate) fn get_scon(scon: &hir::SCon) -> Ty {
  match scon {
    hir::SCon::Int(_) => Ty::INT,
    hir::SCon::Real(_) => Ty::REAL,
    hir::SCon::Word(_) => Ty::WORD,
    hir::SCon::Char(_) => Ty::CHAR,
    hir::SCon::String(_) => Ty::STRING,
  }
}

/// sml_def(6), sml_def(39), sml_def(49)
pub(crate) fn record<T, F, I>(st: &mut St, rows: &[(hir::Lab, T)], idx: I, mut f: F) -> RecordTy
where
  T: Copy,
  F: FnMut(&mut St, &hir::Lab, T) -> Ty,
  I: Into<hir::Idx>,
{
  let mut ty_rows = RecordTy::new();
  let idx = idx.into();
  for (lab, val) in rows {
    let ty = f(st, lab, *val);
    match ty_rows.insert(lab.clone(), ty) {
      None => {}
      Some(_) => st.err(idx, ErrorKind::DuplicateLab(lab.clone())),
    }
  }
  ty_rows
}

/// uses the `names` to traverse through the `StrEnv`s of successive `env`s, returning either the
/// final `env` or the first name that was unbound.
pub(crate) fn get_env<'e, 'n, I>(mut env: &'e Env, names: I) -> Result<&'e Env, &'n hir::Name>
where
  I: IntoIterator<Item = &'n hir::Name>,
{
  for name in names {
    match env.str_env.get(name) {
      None => return Err(name),
      Some(x) => env = x,
    }
  }
  Ok(env)
}

/// substitute any meta type variables in `ty` with their types in `subst`.
///
/// meta variables not defined by the `subst` are left alone.
pub(crate) fn apply(subst: &Subst, ty: &mut Ty) {
  match ty {
    Ty::None | Ty::BoundVar(_) | Ty::FixedVar(_) => {}
    Ty::MetaVar(mv) => match subst.get(mv) {
      None | Some(SubstEntry::Kind(_)) => {}
      Some(SubstEntry::Solved(t)) => {
        let mut t = t.clone();
        apply(subst, &mut t);
        *ty = t;
      }
    },
    Ty::Record(rows) => {
      for ty in rows.values_mut() {
        apply(subst, ty);
      }
    }
    Ty::Con(args, _) => {
      for ty in args.iter_mut() {
        apply(subst, ty);
      }
    }
    Ty::Fn(param, res) => {
      apply(subst, param);
      apply(subst, res);
    }
  }
}

/// instantiates the type scheme's type with new meta type vars, according to the bound vars of the
/// type scheme.
pub(crate) fn instantiate(st: &mut St, ty_scheme: TyScheme) -> Ty {
  let subst: Vec<_> = ty_scheme
    .bound_vars
    .kinds()
    .map(|x| {
      let mv = st.gen_meta_var();
      if let Some(k) = x {
        let k = SubstEntry::Kind(k.clone());
        assert!(st.subst().insert(mv.clone(), k).is_none())
      }
      Ty::MetaVar(mv)
    })
    .collect();
  let mut ty = ty_scheme.ty;
  apply_bv(&subst, &mut ty);
  apply(st.subst(), &mut ty);
  ty
}

/// like [`apply`], but for bound type variables.
///
/// unlike `apply`, all bound variables must be defined by the subst.
pub(crate) fn apply_bv(subst: &[Ty], ty: &mut Ty) {
  match ty {
    Ty::None | Ty::MetaVar(_) | Ty::FixedVar(_) => {}
    Ty::BoundVar(bv) => *ty = bv.index_into(subst).clone(),
    Ty::Record(rows) => {
      for ty in rows.values_mut() {
        apply_bv(subst, ty);
      }
    }
    Ty::Con(args, _) => {
      for ty in args.iter_mut() {
        apply_bv(subst, ty);
      }
    }
    Ty::Fn(param, res) => {
      apply_bv(subst, param);
      apply_bv(subst, res);
    }
  }
}

/// inserts (name, val) into the map, but returns Some(e) if name was already a key, where e is an
/// error describing this transgression.
pub(crate) fn ins_no_dupe<V>(
  map: &mut FxHashMap<hir::Name, V>,
  name: hir::Name,
  val: V,
  item: Item,
) -> Option<ErrorKind> {
  map
    .insert(name.clone(), val)
    .map(|_| ErrorKind::Duplicate(item, name))
}

pub(crate) fn ins_check_name<V>(
  map: &mut FxHashMap<hir::Name, V>,
  name: hir::Name,
  val: V,
  item: Item,
) -> Option<ErrorKind> {
  let no = matches!(
    name.as_str(),
    "true" | "false" | "nil" | "::" | "ref" | "=" | "it"
  );
  no.then(|| ErrorKind::InvalidRebindName(name.clone()))
    .or_else(|| ins_no_dupe(map, name, val, item))
}

/// returns either the ty info in the `env` reached by the `path` or an error describing why we
/// failed to do so.
pub(crate) fn get_ty_info<'e>(env: &'e Env, path: &hir::Path) -> Result<&'e TyInfo, ErrorKind> {
  get_ty_info_raw(env, path.structures().iter(), path.last())
}

pub(crate) fn get_ty_info_raw<'e, 'n, S>(
  env: &'e Env,
  structures: S,
  last: &'n hir::Name,
) -> Result<&'e TyInfo, ErrorKind>
where
  S: IntoIterator<Item = &'n hir::Name>,
{
  match get_env(env, structures) {
    Ok(got_env) => match got_env.ty_env.get(last) {
      Some(ty_info) => Ok(ty_info),
      None => Err(ErrorKind::Undefined(Item::Ty, last.clone())),
    },
    Err(name) => Err(ErrorKind::Undefined(Item::Struct, name.clone())),
  }
}
