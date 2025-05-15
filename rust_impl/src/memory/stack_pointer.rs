use nix::sys::ptrace;
use nix::sys::wait::waitpid;
use nix::unistd::Pid;
use std::fs::File;
use std::io::Read;

use libc::{user_regs_struct, c_long};

pub trait StackPointerReader: Send + Sync {
    fn read(&self, pid: i32) -> usize;
    fn box_clone(&self) -> Box<dyn StackPointerReader>;
}

impl Clone for Box<dyn StackPointerReader> {
    fn clone(&self) -> Box<dyn StackPointerReader> {
        self.box_clone()
    }
}

#[derive(Clone)]
pub struct PtraceStackPointerReader;

impl StackPointerReader for PtraceStackPointerReader {
    fn read(&self, pid: i32) -> usize {
        let target = Pid::from_raw(pid);

        // Attach to the process
        ptrace::attach(target).expect("Failed to ptrace attach");
        waitpid(target, None).expect("Wait for stop failed");

        // Read registers
        let regs: user_regs_struct = unsafe {
            let mut regs: user_regs_struct = std::mem::zeroed();
            let result = libc::ptrace(libc::PTRACE_GETREGS, pid, std::ptr::null_mut::<c_long>(), &mut regs);
            if result == -1 {
                panic!("Failed to get registers via ptrace");
            }
            regs
        };

        // Detach
        ptrace::detach(target, None).expect("Failed to ptrace detach");

        // Return the stack pointer (rsp) value
        regs.rsp as usize
    }

    fn box_clone(&self) -> Box<dyn StackPointerReader> {
        Box::new(Self {})
    }
}

#[derive(Clone)]
pub struct SyscallStackPointerReader;

impl StackPointerReader for SyscallStackPointerReader {
    fn read(&self, pid: i32) -> usize {
        let path = format!("/proc/{}/syscall", pid);
        let mut file = File::open(&path).expect("Failed to open /proc/[pid]/syscall");
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        let tokens: Vec<&str> = contents.split_whitespace().collect();
        let sp = tokens.iter().rev().find(|s| s.starts_with("0x")).unwrap();
        usize::from_str_radix(sp.trim_start_matches("0x"), 16).unwrap()
    }

    fn box_clone(&self) -> Box<dyn StackPointerReader> {
        Box::new(Self {})
    }
}
