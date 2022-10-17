use crate::types::{Class, CmFile, Error, ErrorKind, ParsedPath, PathKind, Result, Root};
use text_size_util::WithRange;

pub(crate) fn get(root: Root) -> Result<CmFile> {
  let mut paths = Vec::<WithRange<ParsedPath>>::new();
  for member in root.members {
    let kind = match member.class() {
      Some(class) => match class.val {
        Class::Sml => PathKind::Sml,
        Class::Cm => PathKind::Cm,
        Class::Other(s) => {
          return Err(Error::new(ErrorKind::UnsupportedClass(member.pathname.val, s), class.range))
        }
      },
      None => {
        return Err(Error::new(
          ErrorKind::CouldNotDetermineClass(member.pathname.val),
          member.pathname.range,
        ))
      }
    };
    paths.push(WithRange {
      val: ParsedPath { path: member.pathname.val, kind },
      range: member.pathname.range,
    });
  }
  Ok(CmFile { exports: root.exports, paths })
}
