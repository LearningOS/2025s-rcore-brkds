//! Implementation of [`PageTableEntry`] and [`PageTable`].

use crate::config::PAGE_SIZE;

use super::{frame_alloc, FrameTracker, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;

bitflags! {
    /// page table entry flags
    pub struct PTEFlags: u8 {
        /// Valid
        const V = 1 << 0;
        /// Readable
        const R = 1 << 1;
        /// Writable
        const W = 1 << 2;
        /// eXecutable
        const X = 1 << 3;
        /// User
        const U = 1 << 4;
        /// Global
        const G = 1 << 5;
        /// Accessed
        const A = 1 << 6;
        /// Dirty
        const D = 1 << 7;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
/// page table entry structure
pub struct PageTableEntry {
    /// bits of page table entry
    pub bits: usize,
}

impl PageTableEntry {
    /// Create a new page table entry
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }
    /// Create an empty page table entry
    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }
    /// Get the physical page number from the page table entry
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }
    /// Get the flags from the page table entry
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    /// The page pointered by page table entry is valid?
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
    /// The page pointered by page table entry is readable?
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
    /// The page pointered by page table entry is writable?
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    /// The page pointered by page table entry is executable?
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

/// page table structure
pub struct PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
}
// impl Drop for PageTable {
//     fn drop(&mut self) {
//         println!("[PageTable] Dropping, frames.len() = {:?}", self.frames);
//     }
// }
/// Assume that it won't oom when creating/mapping.
impl PageTable {
    /// Create a new page table
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }
    /// Temporarily used to get arguments from user space.
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }
    /// Find PageTableEntry by VirtPageNum, create a frame for a 4KB page table if not exist
    pub fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }
    /// Find PageTableEntry by VirtPageNum
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }
    /// set the map between virtual page number and physical page number
    #[allow(unused)]
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        // println!("PTE before mapping (VPN: {:?}): {:?}", vpn, (*pte).flags());
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
         //println!("PTE after mapping (VPN: {:?}): {:?}", vpn, (*pte).flags());
    }
    /// remove the map between virtual page number and physical page number
    #[allow(unused)]
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }
    /// get the page table entry from the virtual page number
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }
    /// get the token from the page table
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
    /// get the root physical page number from the page table
    pub fn root_ppn(&self) -> PhysPageNum {
        self.root_ppn
    }
}

/// Translate&Copy a ptr[u8] array with LENGTH len to a mutable u8 Vec through page table
pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}
/// Copy data from kernel space to user space
pub fn copy_to_user<T>(token: usize,user_ptr: *mut T,data: &T,) -> isize{
    let data_bytes = unsafe {
        core::slice::from_raw_parts(
            data as *const T as *const u8,
            core::mem::size_of::<T>()
        )
    };
    if ((user_ptr as usize) & 0xffff_8000_0000_0000) != 0||!check_user_range(token, user_ptr as usize, data_bytes.len(), PTEFlags::R | PTEFlags::W | PTEFlags::U)
    {   return -1; }
    let user_slices = translated_byte_buffer(
        token,
        user_ptr as *const u8,
        data_bytes.len()
    );
    
    let mut bytes_copied = 0;
    for slice in user_slices {
        let copy_len = slice.len().min(data_bytes.len() - bytes_copied);
        slice[..copy_len].copy_from_slice(&data_bytes[bytes_copied..bytes_copied+copy_len]);
        bytes_copied += copy_len;
    }
    
    if bytes_copied == data_bytes.len() {
        0
    } else {
        -1
    }
}

/// read from user
pub fn read_user<T: Copy>(token: usize, user_ptr: *const T) -> Result<T,isize> {
    // 创建未初始化内存（使用完全限定路径）
    let mut data = ::core::mem::MaybeUninit::<T>::uninit();
    let data_bytes = unsafe {
        ::core::slice::from_raw_parts_mut(
            data.as_mut_ptr() as *mut u8,
            ::core::mem::size_of::<T>()
        )
    };
    println!("user_ptr is {:?}",user_ptr);
    if  ((user_ptr as usize) & 0xffff_8000_0000_0000) != 0||!check_user_range(token, user_ptr as usize, data_bytes.len(), PTEFlags::R | PTEFlags::U)
    {   println!("user_ptr is {:?}",user_ptr);return Err(-1); }
    println!("--user_ptr is {:?}",user_ptr);
    // 获取用户内存切片（假设 translated_byte_buffer 在当前crate根）
    let user_slices = translated_byte_buffer(
        token,
        user_ptr as *const u8,
        ::core::mem::size_of::<T>()
    );
    // 使用切片风格复制数据
    let mut bytes_copied = 0;
    for slice in user_slices {
        let copy_len = slice.len().min(data_bytes.len() - bytes_copied);
        data_bytes[bytes_copied..bytes_copied+copy_len]
            .copy_from_slice(&slice[..copy_len]);
        bytes_copied += copy_len;
    }
    
    unsafe { Ok(data.assume_init()) }
}



/// 检查虚拟地址范围是否可读、可写并对用户有效
pub fn check_user_range(token: usize, start: usize, len: usize, flags: PTEFlags) -> bool {
    let page_table = PageTable::from_token(token);
    let mut current = VirtAddr(start).floor();
    let end = (start + len-1)/PAGE_SIZE+1;
    let rlen:usize = end-start/PAGE_SIZE;
    println!("1current is {:?} {}",current,rlen);
    for _i in 0..rlen {
        if let Some(pte) = page_table.translate(current) {
            // 检查 PTE 是否满足指定的权限
             println!("PTE-------------- (VPN: {:?}): {:?}", current, pte.flags());
            if !pte.is_valid() || (pte.flags() & flags) != flags {
                return false; // 不满足权限
            }
        } else {
            println!("VPN {:?} is not mapped", current);
            return false; // 地址未映射
        }
        current.step();
        println!("current is {:?}",current);
        
    }

    true
}


