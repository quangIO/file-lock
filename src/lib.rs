//! File locking via POSIX advisory record locks.
//!
//! This crate provides the facility to lock and unlock a file following the advisory record lock
//! scheme as specified by UNIX IEEE Std 1003.1-2001 (POSIX.1) via `fcntl()`.
//!

#![feature(libc)]
extern crate libc;

use std::ffi::CString;

/// Represents a lock on a file.
pub struct Lock {
  _fd: i32,
}

/// Represents the error that occured while trying to lock or unlock a file.
pub enum Error {
  /// caused when the filename is invalid as it contains a nul byte.
  InvalidFilename,
  /// caused when the error occurred at the filesystem layer (see
  /// [errno](https://crates.io/crates/errno)).
  Errno(i32),
}

#[repr(C)]
struct c_result {
  _fd:    i32,
  _errno: i32,
}

macro_rules! _create_lock_type {
  ($lock_type:ident, $c_lock_type:ident) => {
    extern {
      fn $c_lock_type(filename: *const libc::c_char) -> c_result;
    }

    /// Locks the specified file.
    ///
    /// The `lock()` and `lock_wait()` functions try to perform a lock on the specified file. The
    /// difference between the two are what they do when another process has a lock on the same
    /// file:
    ///
    /// * lock() - immediately return with an `Errno` error.
    /// * lock_wait() - waits (i.e. blocks the running thread) for the current owner of the lock to
    /// relinquish the lock.
    ///
    /// # Example
    ///
    /// ```
    /// use file_lock::*;
    /// use file_lock::Error::*;
    ///
    /// let l = lock("/tmp/file-lock-test");
    ///
    /// match l {
    ///     Ok(_)  => println!("Got lock"),
    ///     Err(e) => match e {
    ///         InvalidFilename => println!("Invalid filename"),
    ///         Errno(i)        => println!("Got filesystem error {}", i),
    ///     }
    /// }
    /// ```
    pub fn $lock_type(filename: &str) -> Result<Lock, Error> {
      let raw_filename = CString::new(filename);

      if raw_filename.is_err() {
        return Err(Error::InvalidFilename);
      }

      unsafe {
        let my_result = $c_lock_type(raw_filename.unwrap().as_ptr());

        return match my_result._fd {
          -1 => Err(Error::Errno(my_result._errno)),
           _ => Ok(Lock{_fd: my_result._fd}),
        }
      }
    }
  };
}

_create_lock_type!(lock, c_lock);
_create_lock_type!(lock_wait, c_lock_wait);

extern {
  fn c_unlock(_fd: i32) -> c_result;
}

/// Unlocks the file held by `Lock`.
///
/// In reality, you shouldn't need to call `unlock()`. As `Lock` implements the `Drop` trait, once
/// the `Lock` reference goes out of scope, `unlock()` will be called automatically.
///
/// # Example
///
/// ```
/// use file_lock::*;
/// use file_lock::Error::*;
///
/// let l = lock("/tmp/file-lock-test").ok().unwrap();
///
/// println!("Unlocking...");
/// unlock(&l);
/// ```
pub fn unlock(lock: &Lock) -> Result<bool, Error> {
  unsafe {
    let my_result = c_unlock(lock._fd);

    return match my_result._fd {
      -1 => Err(Error::Errno(my_result._errno)),
       _ => Ok(true),
    }
  }
}

#[allow(unused_must_use)]
impl Drop for Lock {
  fn drop(&mut self) {
    unlock(self);
  }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::Error::*;

    //
    // unfortunately we can't abstract this out for lock() and lock_wait()
    // into a macro because string concat doesn't exist
    //

    // lock_wait() tests

    #[test]
    fn lock_invalid_filename() {
        assert_eq!(_lock("null\0inside"), "invalid");
    }

    #[test]
    fn lock_errno() {
        assert_eq!(_lock(""), "errno");
    }

    #[test]
    fn lock_ok() {
        assert_eq!(_lock("/tmp/file-lock-test"), "ok");
    }

    fn _lock(filename: &str) -> &str {
        let l = lock(filename);

        match l {
            Ok(_)  => "ok",
            Err(e) => match e {
                InvalidFilename => "invalid",
                Errno(_)        => "errno",
            }
        }
    }

    // lock_wait() tests

    #[test]
    fn lock_wait_invalid_filename() {
        assert_eq!(_lock_wait("null\0inside"), "invalid");
    }

    #[test]
    fn lock_wait_errno() {
        assert_eq!(_lock_wait(""), "errno");
    }

    #[test]
    fn lock_wait_ok() {
        assert_eq!(_lock_wait("/tmp/file-lock-test"), "ok");
    }

    fn _lock_wait(filename: &str) -> &str {
        let l = lock_wait(filename);

        match l {
            Ok(_)  => "ok",
            Err(e) => match e {
                InvalidFilename => "invalid",
                Errno(_)        => "errno",
            }
        }
    }

    // unlock()

    //
    // fcntl() will only allow us to hold a single lock on a file at a time
    // so this test can't work :(
    //
    // #[test]
    // fn unlock_error() {
    //     let l1 = lock("/tmp/file-lock-test");
    //     let l2 = lock("/tmp/file-lock-test");
    //
    //     assert!(l1.is_ok());
    //     assert!(l2.is_err());
    // }
    //

    #[test]
    fn unlock_ok() {
        let l        = lock_wait("/tmp/file-lock-test");
        let unlocked = l.ok().unwrap();

        assert!(unlock(&unlocked).is_ok(), true);
    }
}
