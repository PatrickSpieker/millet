use std::fmt;

/// returns a char iterator that when collected could be a name for a type variable.
pub(crate) fn ty_var_name(equality: bool, idx: usize) -> impl Iterator<Item = char> {
  let ticks = if equality { 2 } else { 1 };
  std::iter::repeat('\'').take(ticks).chain(idx_to_name(idx))
}

pub(crate) fn idx_to_name(idx: usize) -> impl Iterator<Item = char> {
  let alpha = (b'z' - b'a') as usize;
  let quot = idx / alpha;
  let rem = idx % alpha;
  let ch = char::from((rem as u8) + b'a');
  std::iter::repeat(ch).take(quot + 1)
}

pub(crate) fn comma_seq<I, T>(f: &mut fmt::Formatter<'_>, iter: I) -> fmt::Result
where
  I: Iterator<Item = T>,
  T: fmt::Display,
{
  sep_seq(f, ", ", iter)
}

pub(crate) fn sep_seq<I, T>(f: &mut fmt::Formatter<'_>, sep: &str, mut iter: I) -> fmt::Result
where
  I: Iterator<Item = T>,
  T: fmt::Display,
{
  if let Some(x) = iter.next() {
    x.fmt(f)?;
  }
  for x in iter {
    f.write_str(sep)?;
    x.fmt(f)?;
  }
  Ok(())
}
