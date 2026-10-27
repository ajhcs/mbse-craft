//! Explicit directory-relative read capability. No discovery, writes or link traversal.
use std::path::{Path,Component};
#[cfg(unix)]
use std::{fs::File,io::Read,os::{fd::{AsRawFd,FromRawFd},unix::ffi::OsStrExt},ffi::CString};
pub struct Directory {
    #[cfg(unix)]
    root:File,
}
#[cfg(unix)]
fn open_at(parent:&File,name:&std::ffi::OsStr,directory:bool,context:&str)->Result<File,String>{
    let name=CString::new(name.as_bytes()).map_err(|_|format!("{context}_PATH_NUL"))?;
    let flags=libc::O_RDONLY|libc::O_CLOEXEC|libc::O_NOFOLLOW|if directory{libc::O_DIRECTORY}else{libc::O_NONBLOCK};
    let fd=unsafe{libc::openat(parent.as_raw_fd(),name.as_ptr(),flags)};
    if fd<0{let e=std::io::Error::last_os_error();return Err(if e.kind()==std::io::ErrorKind::NotFound{format!("{context}_FILE_MISSING: {e}")}else{format!("{context}_INPUT_IO: {e}")});}
    // Each successful openat returns one exclusively owned descriptor.
    Ok(unsafe{File::from_raw_fd(fd)})
}
impl Directory {
    pub fn open(path:&Path)->Result<Self,String>{
        if !path.is_absolute(){return Err("ROOTED_INPUT_ABSOLUTE_DIRECTORY_REQUIRED".into());}
        #[cfg(unix)]{
            use std::os::unix::fs::OpenOptionsExt;
            let mut root=std::fs::OpenOptions::new().read(true).custom_flags(libc::O_DIRECTORY|libc::O_NOFOLLOW|libc::O_CLOEXEC).open("/").map_err(|e|e.to_string())?;
            for part in path.components(){match part{Component::RootDir=>{},Component::Normal(name)=>root=open_at(&root,name,true,"ROOTED_INPUT_DIRECTORY")?,_=>return Err("ROOTED_INPUT_EXACT_DIRECTORY_REQUIRED".into())}}
            Ok(Self{root})
        }
        #[cfg(not(unix))]{let _=path;Err("ROOTED_INPUT_PLATFORM_UNQUALIFIED: directory capability unavailable".into())}
    }
    /// Every ancestor and the leaf is opened relative to retained descriptors
    /// with NOFOLLOW. Swapping an ancestor cannot redirect later path lookup.
    pub fn read(&self,relative:&str,limit:usize,context:&str)->Result<Vec<u8>,String>{
        if relative.is_empty()||relative.len()>512||relative.starts_with('/')||relative.contains(['\\',':','\0'])||relative.chars().any(char::is_control)||relative.split('/').any(|p|p.is_empty()||p=="."||p==".."){return Err(format!("{context}_EXACT_RELATIVE_PATH_REQUIRED"));}
        #[cfg(unix)]{
            let mut parent=self.root.try_clone().map_err(|e|e.to_string())?;
            let names=Path::new(relative).components().collect::<Vec<_>>();
            for (i,part)in names.iter().enumerate(){let Component::Normal(name)=part else{return Err(format!("{context}_EXACT_RELATIVE_PATH_REQUIRED"));};if i+1<names.len(){parent=open_at(&parent,name,true,context)?;}else{
                let mut file=open_at(&parent,name,false,context)?;let before=file.metadata().map_err(|e|e.to_string())?;
                if !super::regular(&before){return Err(format!("{context}_INPUT_NOT_REGULAR"));}if before.len()>limit as u64{return Err(format!("{context}_FILE_LIMIT"));}
                let mut bytes=Vec::new();file.by_ref().take(limit as u64+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
                if bytes.len()>limit{return Err(format!("{context}_FILE_LIMIT"));}let after=file.metadata().map_err(|e|e.to_string())?;if !super::same_identity(&before,&after){return Err(format!("{context}_INPUT_CHANGED"));}return Ok(bytes);
            }}Err(format!("{context}_EXACT_RELATIVE_PATH_REQUIRED"))
        }
        #[cfg(not(unix))]{let _=(limit,context);Err("ROOTED_INPUT_PLATFORM_UNQUALIFIED".into())}
    }
}
