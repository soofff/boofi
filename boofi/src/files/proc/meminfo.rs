use crate::files::prelude::*;

#[derive(Debug, Serialize, PartialEq, Description)]
pub(crate) struct Meminfo {
    mem_total: usize,
    mem_free: usize,
    mem_available: usize,
    buffers: usize,
    cached: usize,
    swap_cached: usize,
    active: usize,
    inactive: usize,
    active_anon: usize,
    inactive_anon: usize,
    active_file: usize,
    inactive_file: usize,
    unevictable: usize,
    mlocked: usize,
    swap_total: usize,
    swap_free: usize,
    dirty: usize,
    writeback: usize,
    anon_pages: usize,
    mapped: usize,
    shmem: usize,
    k_reclaimable: usize,
    slab: usize,
    s_reclaimable: usize,
    s_unreclaim: usize,
    kernel_stack: usize,
    page_tables: usize,
    nfs_unstable: usize,
    bounce: usize,
    writeback_tmp: usize,
    commit_limit: usize,
    committed_as: usize,
    vmalloc_total: usize,
    vmalloc_used: usize,
    vmalloc_chunk: usize,
    percpu: usize,
    hardware_corrupted: usize,
    anon_huge_pages: usize,
    shmem_huge_pages: usize,
    shmem_pmd_mapped: usize,
    file_huge_pages: usize,
    file_pmd_mapped: usize,
    huge_pages_total: usize,
    huge_pages_free: usize,
    huge_pages_rsvd: usize,
    huge_pages_surp: usize,
    hugepagesize: usize,
    hugetlb: usize,
    direct_map4k: usize,
    direct_map2m: usize,
}

impl Meminfo {
    fn value(s: &mut Vec<Vec<&str>>) -> Resul<usize> {
        s.remove(0).remove(0).parse().map_err(Into::into)
    }

    pub(crate) fn parse(content: &str) -> Resul<Self> {
        let mut s: Vec<Vec<&str>> = content.split('\n')
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.split(['\t', ' ', ':'])
                    .filter(|s| !s.is_empty())
                    .skip(1)
                    .collect()
            })
            .collect();

        // map and convert by assume order is always same
        Ok(Self {
            mem_total: Self::value(&mut s)?,
            mem_free: Self::value(&mut s)?,
            mem_available: Self::value(&mut s)?,
            buffers: Self::value(&mut s)?,
            cached: Self::value(&mut s)?,
            swap_cached: Self::value(&mut s)?,
            active: Self::value(&mut s)?,
            inactive: Self::value(&mut s)?,
            active_anon: Self::value(&mut s)?,
            inactive_anon: Self::value(&mut s)?,
            active_file: Self::value(&mut s)?,
            inactive_file: Self::value(&mut s)?,
            unevictable: Self::value(&mut s)?,
            mlocked: Self::value(&mut s)?,
            swap_total: Self::value(&mut s)?,
            swap_free: Self::value(&mut s)?,
            dirty: Self::value(&mut s)?,
            writeback: Self::value(&mut s)?,
            anon_pages: Self::value(&mut s)?,
            mapped: Self::value(&mut s)?,
            shmem: Self::value(&mut s)?,
            k_reclaimable: Self::value(&mut s)?,
            slab: Self::value(&mut s)?,
            s_reclaimable: Self::value(&mut s)?,
            s_unreclaim: Self::value(&mut s)?,
            kernel_stack: Self::value(&mut s)?,
            page_tables: Self::value(&mut s)?,
            nfs_unstable: Self::value(&mut s)?,
            bounce: Self::value(&mut s)?,
            writeback_tmp: Self::value(&mut s)?,
            commit_limit: Self::value(&mut s)?,
            committed_as: Self::value(&mut s)?,
            vmalloc_total: Self::value(&mut s)?,
            vmalloc_used: Self::value(&mut s)?,
            vmalloc_chunk: Self::value(&mut s)?,
            percpu: Self::value(&mut s)?,
            hardware_corrupted: Self::value(&mut s)?,
            anon_huge_pages: Self::value(&mut s)?,
            shmem_huge_pages: Self::value(&mut s)?,
            shmem_pmd_mapped: Self::value(&mut s)?,
            file_huge_pages: Self::value(&mut s)?,
            file_pmd_mapped: Self::value(&mut s)?,
            huge_pages_total: Self::value(&mut s)?,
            huge_pages_free: Self::value(&mut s)?,
            huge_pages_rsvd: Self::value(&mut s)?,
            huge_pages_surp: Self::value(&mut s)?,
            hugepagesize: Self::value(&mut s)?,
            hugetlb: Self::value(&mut s)?,
            direct_map4k: Self::value(&mut s)?,
            direct_map2m: Self::value(&mut s)?,
        })
    }
}


pub(crate) struct MeminfoFile {
    path: String,
}

#[async_trait]
impl File for MeminfoFile {
    type Output = Meminfo;
    type Input = ();

    fn new(path: &str) -> Self {
        Self {
            path: path.into(),
        }
    }

    async fn read(&self, system: &System) -> Resul<Self::Output> {
        Meminfo::parse(system
            .read_to_string(self.path()).await?.as_str())
    }
    fn path(&self) -> &str {
        &self.path
    }
}


#[derive(Clone)]
pub(crate) struct MeminfoBuilder;

impl FileBuilder for MeminfoBuilder {
    type File = MeminfoFile;

    const NAME: &'static str = "meminfo";
    const DESCRIPTION: &'static str = "Memory information";
    const CAPABILITIES: &'static [Capability] = &[Capability::Read];

    fn patterns(&self) -> &[FileMatchPattern] {
        lazy_static! {
            static ref PATTERN: [FileMatchPattern;1] = [FileMatchPattern::new_path("/proc/meminfo", &[Os::LinuxAny])];
        }

        PATTERN.as_slice()
    }

    fn examples(&self) -> &[FileExample] {
        lazy_static! {
            static ref EAMPLES: [FileExample;1] = [
                FileExample::new_get("Simple example",
                    vec![Meminfo {
                       mem_total:1,
                        mem_free:2,
                        mem_available:3,
                        buffers:4,
                        cached:5,
                        swap_cached:6,
                        active:7,
                        inactive:8,
                        active_anon:9,
                        inactive_anon:0,
                        active_file:1,
                        inactive_file:2,
                        unevictable:3,
                        mlocked:4,
                        swap_total:5,
                        swap_free:67890,
                        dirty:1,
                        writeback:2,
                        anon_pages:3,
                        mapped:4,
                        shmem:5,
                        k_reclaimable:6,
                        slab:7,
                        s_reclaimable:8,
                        s_unreclaim:9,
                        kernel_stack:0,
                        page_tables:1,
                        nfs_unstable:22,
                        bounce:333,
                        writeback_tmp:4444,
                        commit_limit:55555,
                        committed_as:666666,
                        vmalloc_total:7777777,
                        vmalloc_used:88888888,
                        vmalloc_chunk:999999999,
                        percpu:0,
                        hardware_corrupted:1,
                        anon_huge_pages:2,
                        shmem_huge_pages:3,
                        shmem_pmd_mapped:4,
                        file_huge_pages:5,
                        file_pmd_mapped:6,
                        huge_pages_total:7,
                        huge_pages_free:8,
                        huge_pages_rsvd:9,
                        huge_pages_surp:0,
                        hugepagesize:1,
                        hugetlb:2,
                        direct_map4k:3,
                        direct_map2m:4,
                       }]
                )
            ];
        }

        EAMPLES.as_slice()
    }
}

#[cfg(test)]
mod test {
    use crate::files::meminfo::Meminfo;
    use crate::utils::test::read_test_resources;

    #[test]
    fn test_parse() {
        assert_eq!(Meminfo::parse(&read_test_resources("meminfo")).unwrap(), Meminfo {
            mem_total: 8128068,
            mem_free: 1577652,
            mem_available: 4473712,
            buffers: 104308,
            cached: 2970804,
            swap_cached: 0,
            active: 958308,
            inactive: 5118904,
            active_anon: 1400,
            inactive_anon: 3024092,
            active_file: 956908,
            inactive_file: 2094812,
            unevictable: 32,
            mlocked: 32,
            swap_total: 2097148,
            swap_free: 2097148,
            dirty: 88080,
            writeback: 0,
            anon_pages: 3002140,
            mapped: 1077960,
            shmem: 29200,
            k_reclaimable: 126416,
            slab: 202744,
            s_reclaimable: 126416,
            s_unreclaim: 76328,
            kernel_stack: 13232,
            page_tables: 29204,
            nfs_unstable: 0,
            bounce: 0,
            writeback_tmp: 0,
            commit_limit: 6161180,
            committed_as: 6964468,
            vmalloc_total: 34359738367,
            vmalloc_used: 44572,
            vmalloc_chunk: 0,
            percpu: 3504,
            hardware_corrupted: 0,
            anon_huge_pages: 0,
            shmem_huge_pages: 0,
            shmem_pmd_mapped: 0,
            file_huge_pages: 0,
            file_pmd_mapped: 0,
            huge_pages_total: 0,
            huge_pages_free: 0,
            huge_pages_rsvd: 0,
            huge_pages_surp: 0,
            hugepagesize: 2048,
            hugetlb: 0,
            direct_map4k: 221120,
            direct_map2m: 8167424,
        });
    }
}