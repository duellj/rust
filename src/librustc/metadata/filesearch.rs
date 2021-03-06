// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


// A module for searching for libraries
// FIXME (#2658): I'm not happy how this module turned out. Should
// probably just be folded into cstore.

use core::prelude::*;

use core::option;
use core::os;
use core::result::Result;
use core::result;
use core::str;

pub type pick<'self, T> = &'self fn(path: &Path) -> Option<T>;

pub fn pick_file(file: Path, path: &Path) -> Option<Path> {
    if path.file_path() == file { option::Some(copy *path) }
    else { option::None }
}

pub trait FileSearch {
    fn sysroot(&self) -> Path;
    fn lib_search_paths(&self) -> ~[Path];
    fn get_target_lib_path(&self) -> Path;
    fn get_target_lib_file_path(&self, file: &Path) -> Path;
}

pub fn mk_filesearch(maybe_sysroot: Option<Path>,
                     target_triple: &str,
                     +addl_lib_search_paths: ~[Path])
                  -> @FileSearch {
    struct FileSearchImpl {
        sysroot: Path,
        addl_lib_search_paths: ~[Path],
        target_triple: ~str
    }
    impl FileSearch for FileSearchImpl {
        fn sysroot(&self) -> Path { /*bad*/copy self.sysroot }
        fn lib_search_paths(&self) -> ~[Path] {
            let mut paths = /*bad*/copy self.addl_lib_search_paths;

            paths.push(
                make_target_lib_path(&self.sysroot,
                                     self.target_triple));
            match get_rustpkg_lib_path_nearest() {
              result::Ok(ref p) => paths.push((/*bad*/copy *p)),
              result::Err(_) => ()
            }
            match get_rustpkg_lib_path() {
              result::Ok(ref p) => paths.push((/*bad*/copy *p)),
              result::Err(_) => ()
            }
            paths
        }
        fn get_target_lib_path(&self) -> Path {
            make_target_lib_path(&self.sysroot, self.target_triple)
        }
        fn get_target_lib_file_path(&self, file: &Path) -> Path {
            self.get_target_lib_path().push_rel(file)
        }
    }

    let sysroot = get_sysroot(maybe_sysroot);
    debug!("using sysroot = %s", sysroot.to_str());
    @FileSearchImpl {
        sysroot: sysroot,
        addl_lib_search_paths: addl_lib_search_paths,
        target_triple: str::from_slice(target_triple)
    } as @FileSearch
}

pub fn search<T:Copy>(filesearch: @FileSearch, pick: pick<T>) -> Option<T> {
    let mut rslt = None;
    for filesearch.lib_search_paths().each |lib_search_path| {
        debug!("searching %s", lib_search_path.to_str());
        for os::list_dir_path(lib_search_path).each |path| {
            debug!("testing %s", path.to_str());
            let maybe_picked = pick(*path);
            if maybe_picked.is_some() {
                debug!("picked %s", path.to_str());
                rslt = maybe_picked;
                break;
            } else {
                debug!("rejected %s", path.to_str());
            }
        }
        if rslt.is_some() { break; }
    }
    return rslt;
}

pub fn relative_target_lib_path(target_triple: &str) -> Path {
    Path(libdir()).push_many([~"rustc",
                              str::from_slice(target_triple),
                              libdir()])
}

fn make_target_lib_path(sysroot: &Path,
                        target_triple: &str) -> Path {
    sysroot.push_rel(&relative_target_lib_path(target_triple))
}

fn get_or_default_sysroot() -> Path {
    match os::self_exe_path() {
      option::Some(ref p) => (*p).pop(),
      option::None => fail!(~"can't determine value for sysroot")
    }
}

fn get_sysroot(maybe_sysroot: Option<Path>) -> Path {
    match maybe_sysroot {
      option::Some(ref sr) => (/*bad*/copy *sr),
      option::None => get_or_default_sysroot()
    }
}

pub fn get_rustpkg_sysroot() -> Result<Path, ~str> {
    result::Ok(get_or_default_sysroot().push_many([libdir(), ~"rustpkg"]))
}

pub fn get_rustpkg_root() -> Result<Path, ~str> {
    match os::getenv(~"RUSTPKG_ROOT") {
        Some(ref _p) => result::Ok(Path((*_p))),
        None => match os::homedir() {
          Some(ref _q) => result::Ok((*_q).push(".rustpkg")),
          None => result::Err(~"no RUSTPKG_ROOT or home directory")
        }
    }
}

pub fn get_rustpkg_root_nearest() -> Result<Path, ~str> {
    do result::chain(get_rustpkg_root()) |p| {
        let cwd = os::getcwd();
        let cwd_rustpkg = cwd.push(".rustpkg");
        let rustpkg_is_non_root_file =
            !os::path_is_dir(&cwd_rustpkg) && cwd_rustpkg != p;
        let mut par_rustpkg = cwd.pop().push(".rustpkg");
        let mut rslt = result::Ok(cwd_rustpkg);

        if rustpkg_is_non_root_file {
            while par_rustpkg != p {
                if os::path_is_dir(&par_rustpkg) {
                    rslt = result::Ok(par_rustpkg);
                    break;
                }
                if par_rustpkg.components.len() == 1 {
                    // We just checked /.rustpkg, stop now.
                    break;
                }
                par_rustpkg = par_rustpkg.pop().pop().push(".rustpkg");
            }
        }
        rslt
    }
}

fn get_rustpkg_lib_path() -> Result<Path, ~str> {
    do result::chain(get_rustpkg_root()) |p| {
        result::Ok(p.push(libdir()))
    }
}

fn get_rustpkg_lib_path_nearest() -> Result<Path, ~str> {
    do result::chain(get_rustpkg_root_nearest()) |p| {
        result::Ok(p.push(libdir()))
    }
}

// The name of the directory rustc expects libraries to be located.
// On Unix should be "lib", on windows "bin"
pub fn libdir() -> ~str {
   let libdir = env!("CFG_LIBDIR");
   if str::is_empty(libdir) {
      fail!(~"rustc compiled without CFG_LIBDIR environment variable");
   }
   libdir
}
