use std::ffi::CString;
use std::ptr;
use std::os::raw::c_int;

use failure::Error;
use nix::errno::Errno;

use crate::fork::forking_syscall;
use crate::lxcseccomp::ProxyMessageBuffer;
use crate::pidfd::PidFd;
use crate::sc_libc_try;
use crate::syscall::SyscallStatus;

/*
 * int quotactl(int cmd, const char *special, int id, caddr_t addr);
 *
 * `special` is a path to the mount point, so we need to be in the process' file system view
 * `addr` will point to a datastructure, so we should read a reasonable amount (a 4k page?) of
 *  memory from there, but only if the sub command type makes use of it.
 *
 *  Actually the libc crate contains most of the structures anyway!
 *
 * Cmd:
 *  QCMD(SubCmd, Type)
 *      Type: USRQUOTA | GRPQUOTA | PRJQUOTA (but we don't need to care)
 * SubCmd:
 *  |          name           addr meaning|
 *        Q_QUOTAON     path to quota file
 *       Q_QUOTAOFF                ignored
 *       Q_GETQUOTA        struct dqblk {}       a page should be sufficient
 *   Q_GETNEXTQUOTA    struct nextdqblk {}       a page should be sufficient
 *       Q_SETQUOTA        struct dqblk {}       a page should be sufficient
 *       Q_SETQUOTA        struct dqblk {}       a page should be sufficient
 *        Q_SETQLIM                              -EOPNOTSUPP: not documented anymore
 *         Q_SETUSE                              -EOPNOTSUPP: not documented anymore
 *        Q_GETINFO       struct dqinfo {}
 *        Q_SETINFO       struct dqinfo {}
 *         Q_GETFMT                [u8; 4]
 *           Q_SYNC                ignored       -EOPNOTSUPP if `special` is NULL!
 *       Q_GETSTATS      struct dqstats {}       -EOPNOTSUPP: obsolete, removed since 2.4.22!
 *
 * xfs stuff:
 *       Q_XQUOTAON           unsigned int
 *      Q_XQUOTAOFF           unsigned int
 *      ...
 *      (we don't actually have xfs containers atm...)
 */

const SUBCMDSHIFT: c_int = 8;

pub async fn quotactl(msg: &ProxyMessageBuffer) -> Result<SyscallStatus, Error> {
    let cmd = msg.arg_int(0)?;
    let special = msg.arg_opt_c_string(1)?;
    // let _id = msg.arg_int(2)?;
    // let _addr = msg.arg_caddr_t(3)?;

    match cmd >> SUBCMDSHIFT {
        libc::Q_QUOTAON => q_quotaon(msg, cmd, special).await,
        _ => Ok(Errno::ENOSYS.into()),
    }
}

pub async fn q_quotaon(
    msg: &ProxyMessageBuffer,
    cmd: c_int,
    special: Option<CString>,
) -> Result<SyscallStatus, Error> {
    let id = msg.arg_int(2)?;
    let addr = msg.arg_caddr_t(3)?;
    let special = special.map(|c| c.as_ptr()).unwrap_or(ptr::null());

    let caps = msg.pid_fd().user_caps()?;
    Ok(forking_syscall(move || {
        let this = PidFd::current()?;
        caps.apply(&this)?;
        let out = sc_libc_try!(unsafe { libc::quotactl(cmd, special, id, addr) });
        Ok(SyscallStatus::Ok(out.into()))
    })
    .await?)
}
