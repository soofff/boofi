use crate::files::prelude::*;

fn string2bool(s: String) -> bool {
    s.contains("yes")
}

#[derive(Serialize, Debug, PartialEq, Description)]
pub(crate) struct CpuInfoDetail {
    processor: usize,
    vendor_id: String,
    cpu_family: usize,
    model: usize,
    model_name: String,
    stepping: usize,
    microcode: String,
    cpu_mhz: f64,
    cache_size: String,
    physical_id: usize,
    siblings: usize,
    core_id: usize,
    cpu_cores: usize,
    apicid: usize,
    initial_apicid: usize,
    fpu: bool,
    fpu_exception: bool,
    cpuid_level: usize,
    wp: bool,
    flags: Vec<String>,
    bugs: Vec<String>,
    bogomips: f64,
    tlb_size: String,
    clflush_size: usize,
    cache_alignment: usize,
    address_sizes: String,
}

impl CpuInfoDetail {
    fn parse(content: &str) -> Resul<Self> {
        let mut f = content.split('\n').map(|l| {
            l.split(':').last().unwrap_or_default().trim().to_string()
        }).collect::<Vec<String>>();

        Ok(Self {
            processor: f.remove(0).parse()?,
            vendor_id: f.remove(0),
            cpu_family: f.remove(0).parse()?,
            model: f.remove(0).parse()?,
            model_name: f.remove(0),
            stepping: f.remove(0).parse()?,
            microcode: f.remove(0),
            cpu_mhz: f.remove(0).parse()?,
            cache_size: f.remove(0),
            physical_id: f.remove(0).parse()?,
            siblings: f.remove(0).parse()?,
            core_id: f.remove(0).parse()?,
            cpu_cores: f.remove(0).parse()?,
            apicid: f.remove(0).parse()?,
            initial_apicid: f.remove(0).parse()?,
            fpu: string2bool(f.remove(0)),
            fpu_exception: string2bool(f.remove(0)),
            cpuid_level: f.remove(0).parse()?,
            wp: string2bool(f.remove(0)),
            flags: f.remove(0).split_whitespace().map(String::from).collect(),
            bugs: f.remove(0).split_whitespace().map(String::from).collect(),
            bogomips: f.remove(0).parse()?,
            tlb_size: f.remove(0),
            clflush_size: f.remove(0).parse()?,
            cache_alignment: f.remove(0).parse()?,
            address_sizes: f.remove(0),
        })
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct CpuInfo;

impl CpuInfo {
    fn parse(content: &str) -> Resul<Vec<CpuInfoDetail>> {
        content.split("\n\n")
            .filter(|s| !s.is_empty())
            .map(CpuInfoDetail::parse)
            .collect()
    }
}

pub(crate) struct CpuinfoFile {
    path: String,
}

#[async_trait]
impl File for CpuinfoFile {
    type Output = Vec<CpuInfoDetail>;
    type Input = ();

    fn new(path: &str) -> Self {
        Self {
            path: path.into(),
        }
    }

    async fn read(&self, system: &System) -> Resul<Self::Output> {
        CpuInfo::parse(&system.read_to_string(self.path()).await?)
    }

    fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Clone)]
pub(crate) struct CpuinfoBuilder;

impl FileBuilder for CpuinfoBuilder {
    type File = CpuinfoFile;

    const NAME: &'static str = "cpuinfo";
    const DESCRIPTION: &'static str = "Get information about processor";
    const CAPABILITIES: &'static [Capability] = &[Capability::Read];

    fn patterns(&self) -> &[FileMatchPattern] {
        lazy_static! {
            static ref PATTERN: [FileMatchPattern; 1] = [FileMatchPattern::new_path("/proc/cpuinfo", &[Os::LinuxAny])];
        }
        PATTERN.as_slice()
    }

    fn examples(&self) -> &[FileExample] {
        lazy_static! {
            static ref FILEEXAMPLE: [FileExample; 1] = [
                FileExample::new_get("Single processor output", CpuInfoDetail {
                        processor: 0,
                        vendor_id: "AMtel".to_string(),
                        cpu_family: 1,
                        model: 2,
                        model_name: "Core i Ryzen".to_string(),
                        stepping: 0,
                        microcode: "0xFFFFFF".to_string(),
                        cpu_mhz: 133.7,
                        cache_size: "1 Mb".to_string(),
                        physical_id: 0,
                        siblings: 0,
                        core_id: 0,
                        cpu_cores: 1,
                        apicid: 0,
                        initial_apicid: 0,
                        fpu: false,
                        fpu_exception: true,
                        cpuid_level: 0,
                        wp: true,
                        flags: vec!["sse".to_string(), "aes".to_string()],
                        bugs: vec![],
                        bogomips: 1234.56,
                        tlb_size: "".to_string(),
                        clflush_size: 0,
                        cache_alignment: 0,
                        address_sizes: "".to_string(),
                    }
                )
            ];
        }

        FILEEXAMPLE.as_slice()
    }
}


#[cfg(test)]
mod test {
    use crate::files::cpuinfo::{CpuInfo, CpuInfoDetail};
    use crate::utils::test::read_test_resources;

    #[test]
    fn test_parse() {
        assert_eq!(CpuInfo::parse(&read_test_resources("cpuinfo")).unwrap(), vec![
            CpuInfoDetail {
                processor: 0,
                vendor_id: "AuthenticAMD".into(),
                cpu_family: 23,
                model: 8,
                model_name: "AMD Ryzen 5 2600X Six-Core Processor".into(),
                stepping: 2,
                microcode: "0xffffffff".into(),
                cpu_mhz: 3600.116,
                cache_size: "512 KB".into(),
                physical_id: 0,
                siblings: 4,
                core_id: 0,
                cpu_cores: 4,
                apicid: 0,
                initial_apicid: 0,
                fpu: true,
                fpu_exception: true,
                cpuid_level: 13,
                wp: true,
                flags: ["fpu", "vme", "de", "pse", "tsc", "msr", "pae", "mce", "cx8", "apic", "sep", "mtrr", "pge", "mca", "cmov", "pat", "pse36", "clflush", "mmx", "fxsr", "sse", "sse2", "ht", "syscall", "nx", "mmxext", "fxsr_opt", "rdtscp", "lm", "constant_tsc", "rep_good", "nopl", "nonstop_tsc", "cpuid", "extd_apicid", "tsc_known_freq", "pni", "pclmulqdq", "ssse3", "cx16", "sse4_1", "sse4_2", "movbe", "popcnt", "aes", "rdrand", "hypervisor", "lahf_lm", "cmp_legacy", "cr8_legacy", "abm", "sse4a", "misalignsse", "3dnowprefetch", "ssbd", "vmmcall", "fsgsbase", "bmi1", "bmi2", "rdseed", "clflushopt", "arat"].iter().map(ToString::to_string).collect(),
                bugs: ["fxsave_leak", "sysret_ss_attrs", "null_seg", "spectre_v1", "spectre_v2", "retbleed", "smt_rsb"].iter().map(ToString::to_string).collect(),
                bogomips: 7200.23,
                tlb_size: "2560 4K pages".into(),
                clflush_size: 64,
                cache_alignment: 64,
                address_sizes: "48 bits physical, 48 bits virtual".into(),
            }, CpuInfoDetail {
                processor: 1,
                vendor_id: "AuthenticAMD".into(),
                cpu_family: 23,
                model: 8,
                model_name: "AMD Ryzen 5 2600X Six-Core Processor".into(),
                stepping: 2,
                microcode: "0xffffffff".into(),
                cpu_mhz: 3600.116,
                cache_size: "512 KB".into(),
                physical_id: 0,
                siblings: 4,
                core_id: 1,
                cpu_cores: 4,
                apicid: 1,
                initial_apicid: 1,
                fpu: true,
                fpu_exception: true,
                cpuid_level: 13,
                wp: true,
                flags: ["fpu", "vme", "de", "pse", "tsc", "msr", "pae", "mce", "cx8", "apic", "sep", "mtrr", "pge", "mca", "cmov", "pat", "pse36", "clflush", "mmx", "fxsr", "sse", "sse2", "ht", "syscall", "nx", "mmxext", "fxsr_opt", "rdtscp", "lm", "constant_tsc", "rep_good", "nopl", "nonstop_tsc", "cpuid", "extd_apicid", "tsc_known_freq", "pni", "pclmulqdq", "ssse3", "cx16", "sse4_1", "sse4_2", "movbe", "popcnt", "aes", "rdrand", "hypervisor", "lahf_lm", "cmp_legacy", "cr8_legacy", "abm", "sse4a", "misalignsse", "3dnowprefetch", "ssbd", "vmmcall", "fsgsbase", "bmi1", "bmi2", "rdseed", "clflushopt", "arat"].iter().map(ToString::to_string).collect(),
                bugs: ["fxsave_leak", "sysret_ss_attrs", "null_seg", "spectre_v1", "spectre_v2", "retbleed", "smt_rsb"].iter().map(ToString::to_string).collect(),
                bogomips: 7200.23,
                tlb_size: "2560 4K pages".into(),
                clflush_size: 64,
                cache_alignment: 64,
                address_sizes: "48 bits physical, 48 bits virtual".into(),
            },
        ]);
    }
}