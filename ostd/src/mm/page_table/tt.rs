
#[cfg(ktest)]
mod tests {
    use crate::{
        mm::{
            frame::FrameMeta,
            kspace::LINEAR_MAPPING_BASE_VADDR,
            page::allocator,
            page_prop::{CachePolicy, PageFlags, PageProperty},
            MAX_USERSPACE_VADDR,
            page_table::*,
        },
        prelude::*,
    };
    use core::mem::ManuallyDrop;

    const PAGE_SIZE: usize = 4096;
    
    // 通用的测试辅助结构体
    struct TestPageTableSetup<M: PageTableMode> {
        pt: PageTable<M>,
        default_prop: PageProperty,
    }

    impl<M: PageTableMode> TestPageTableSetup<M> {
        fn new() -> Self {
            Self {
                pt: PageTable::<M>::empty(),
                default_prop: PageProperty::new(PageFlags::RW, CachePolicy::Writeback),
            }
        }

        fn map_and_verify(&self, vaddr_range: Range<Vaddr>, paddr_range: Range<Paddr>) -> Result<()> {
            unsafe { 
                self.pt.map(&vaddr_range, &paddr_range, self.default_prop)?;
            }
            for vaddr in vaddr_range.start..vaddr_range.end {
                let (paddr, prop) = unsafe { self.pt.query(vaddr).expect("Mapping should exist") };
                assert_eq!(paddr, paddr_range.start + (vaddr - vaddr_range.start));
                assert_eq!(prop, self.default_prop);
            }
        
            Ok(())
        }

        fn protect_and_verify(&self, range: &Range<Vaddr>, new_flags: PageFlags) {
            self.pt.protect(range, |prop| prop.flags = new_flags);
            
            for vaddr in range.start..range.end {
                let (_, prop) = self.pt.query(vaddr).expect("Mapping should exist");
                assert_eq!(prop.flags, new_flags);
            }
        }
    }

    mod basic_mapping_tests {
        use super::*;

        #[ktest]
        fn test_empty_page_table() {
            let setup = TestPageTableSetup::<KernelMode>::new();
            assert_eq!(unsafe { setup.pt.root_paddr() }, 32768);
        }

        #[ktest]
        fn test_create_user_page_table() {
            let kernel_setup = TestPageTableSetup::<KernelMode>::new();
            let user_pt = kernel_setup.pt.create_user_page_table();
            assert_ne!(unsafe { user_pt.root_paddr() }, 0);
        }

        #[ktest]
        fn test_basic_mapping() {
            let setup = TestPageTableSetup::<KernelMode>::new();
            
            let vaddr_range = 0..PAGE_SIZE;
            let paddr_range = 0..PAGE_SIZE;
            assert!(setup.map_and_verify(vaddr_range, paddr_range).is_ok());
        }

        #[ktest]
        fn test_boundary_conditions() {
            let setup = TestPageTableSetup::<UserMode>::new();
            
            assert!(setup.pt.cursor_mut(&(0..0)).is_err());
            assert!(setup.pt.cursor_mut(&(MAX_USERSPACE_VADDR..MAX_USERSPACE_VADDR + PAGE_SIZE)).is_err());
            assert!(setup.pt.cursor_mut(&(1..PAGE_SIZE + 1)).is_err());
        }
    }

    mod protection_tests {
        use super::*;

        #[ktest]
        fn test_protection_changes() {
            let setup = TestPageTableSetup::<UserMode>::new();
            
            let range = 0..PAGE_SIZE * 4;
            let paddr_range = 0..PAGE_SIZE * 4;
            setup.map_and_verify(range.clone(), paddr_range).unwrap();
            
            let protect_range = PAGE_SIZE..PAGE_SIZE * 2;
            setup.protect_and_verify(&protect_range, PageFlags::RX);
        }

        #[ktest]
        fn test_protection_flush_tlb() {
            let setup = TestPageTableSetup::<KernelMode>::new();
            let range = 0..PAGE_SIZE;
            
            let result = unsafe { 
                setup.pt.protect_flush_tlb(&range, |prop| prop.flags = PageFlags::RX) 
            };
            assert!(result.is_ok());
        }
    }

    mod copy_on_write_tests {
        use super::*;
        use crate::Error;

        struct CowTestSetup {
            parent: TestPageTableSetup<UserMode>,
            child: TestPageTableSetup<UserMode>,
        }

        impl CowTestSetup {
            fn new() -> Self {
                Self {
                    parent: TestPageTableSetup::new(),
                    child: TestPageTableSetup::new(),
                }
            }

            fn setup_cow_mapping(&self, range: Range<Vaddr>) -> Result<()> {
                let page = allocator::alloc_single(FrameMeta::default()).ok_or(Error::NoMemory)?;
                let paddr = page.paddr();
                let range_len = range.end - range.start;
                self.parent.map_and_verify(range.clone(), paddr..paddr + range_len)?;
                
                unsafe {
                    let mut child_cursor = self.child.pt.cursor_mut(&range)?;
                    let mut parent_cursor = self.parent.pt.cursor_mut(&range)?;
                    child_cursor.copy_from(&mut parent_cursor, range.len(), &mut |prop: &mut PageProperty| {
                        prop.flags -= PageFlags::W;
                    });
                }
                Ok(())
            }
        }

        #[ktest]
        fn test_cow_basic() {
            let setup = CowTestSetup::new();
            let range = PAGE_SIZE..PAGE_SIZE * 2;
            
            setup.setup_cow_mapping(range.clone()).unwrap();
            
            let parent_mapping = setup.parent.pt.query(range.start).unwrap();
            let child_mapping = setup.child.pt.query(range.start).unwrap();
            assert_eq!(parent_mapping.0, child_mapping.0);
            assert_eq!(child_mapping.1.flags, PageFlags::R);
        }
    }

    mod huge_page_tests {
        use super::*;

        #[ktest]
        fn test_mixed_page_sizes() {
            let setup = TestPageTableSetup::<UserMode>::new();
            
            // 2MB页映射
            let huge_range = PAGE_SIZE * 512..PAGE_SIZE * 512 * 2;
            let huge_paddr = 0..PAGE_SIZE * 512;
            setup.map_and_verify(huge_range, huge_paddr).unwrap();
            
            // 4KB页映射
            let small_range = 0..PAGE_SIZE;
            let small_paddr = PAGE_SIZE * 512 * 2..PAGE_SIZE * 512 * 2 + PAGE_SIZE;
            setup.map_and_verify(small_range, small_paddr).unwrap();
        }

        #[derive(Clone, Debug, Default)]
        struct VeryHugePagingConsts {}

        impl PagingConstsTrait for VeryHugePagingConsts {
            const NR_LEVELS: PagingLevel = 4;
            const BASE_PAGE_SIZE: usize = PAGE_SIZE;
            const ADDRESS_WIDTH: usize = 48;
            const HIGHEST_TRANSLATION_LEVEL: PagingLevel = 3;
            const PTE_SIZE: usize = core::mem::size_of::<PageTableEntry>();
        }

        #[ktest]
        fn test_untracked_large_mapping() {
            let pt = PageTable::<KernelMode, PageTableEntry, VeryHugePagingConsts>::empty();
            const UNTRACKED_OFFSET: usize = crate::mm::kspace::LINEAR_MAPPING_BASE_VADDR;

            let gmult = 512 * 512;
            let from_ppn = gmult - 512..gmult + gmult + 514;
            let to_ppn = gmult - 512 - 512..gmult + gmult - 512 + 514;
            
            let from = UNTRACKED_OFFSET + PAGE_SIZE * from_ppn.start..
                      UNTRACKED_OFFSET + PAGE_SIZE * from_ppn.end;
            let to = PAGE_SIZE * to_ppn.start..PAGE_SIZE * to_ppn.end;
            
            let prop = PageProperty::new(PageFlags::RW, CachePolicy::Writeback);
            unsafe { 
                pt.map(&from, &to, prop).unwrap();
                
                // Verify mappings
                for (item, i) in pt.cursor(&from).unwrap().zip(0..512 + 2 + 2) {
                    let PageTableItem::MappedUntracked { va, pa, len, prop } = item else {
                        panic!("Expected MappedUntracked, got {:#x?}", item);
                    };
                    assert_eq!(pa, va - (from.start - to.start));
                    assert_eq!(prop.flags, PageFlags::RW);
                    
                    if i < 512 + 2 {
                        assert_eq!(va, from.start + i * PAGE_SIZE * 512);
                        assert_eq!(va + len, from.start + (i + 1) * PAGE_SIZE * 512);
                    } else {
                        assert_eq!(
                            va,
                            from.start + (512 + 2) * PAGE_SIZE * 512 + (i - 512 - 2) * PAGE_SIZE
                        );
                        assert_eq!(
                            va + len,
                            from.start + (512 + 2) * PAGE_SIZE * 512 + (i - 512 - 2 + 1) * PAGE_SIZE
                        );
                    }
                }
            }

            let _ = ManuallyDrop::new(pt);
        }
    }

    impl<M: PageTableMode, E: PageTableEntryTrait, C: PagingConstsTrait> PageTable<M, E, C>
    where
        [(); C::NR_LEVELS as usize]:,
    {
        fn protect(&self, range: &Range<Vaddr>, mut op: impl FnMut(&mut PageProperty)) {
            let mut cursor = self.cursor_mut(range).unwrap();
            loop {
                unsafe {
                    if cursor
                        .protect_next(range.end - cursor.virt_addr(), &mut op)
                        .is_none()
                    {
                        break;
                    }
                };
            }
        }
    }
}