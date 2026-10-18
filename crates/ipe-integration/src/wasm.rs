//! A pure metadata interpretation sandbox. No WASI or host imports are linked.
use sha2::{Digest, Sha256};
use std::io::Read;
use wasmi::{Engine, Linker, Memory, Module, Store, StoreLimits, StoreLimitsBuilder, TypedFunc};

const MODULE_BYTES: usize = 262_144;
const FUEL: u64 = 1_000_000;
pub(crate) struct Adapter {
    engine: Engine,
    module: Module,
    pub(crate) digest: String,
}
pub(crate) struct Run {
    store: Store<StoreLimits>,
    memory: Memory,
    classify: TypedFunc<(i32, i32, i32, i32), i32>,
    failed: bool,
}
impl Adapter {
    pub(crate) fn load(path: Option<&str>) -> Result<Self, &'static str> {
        let bytes = match path {
            Some(path) => {
                let mut bytes = Vec::new();
                std::fs::File::open(path)
                    .map_err(|_| "adapter_unavailable")?
                    .take((MODULE_BYTES + 1) as u64)
                    .read_to_end(&mut bytes)
                    .map_err(|_| "adapter_unavailable")?;
                bytes
            },
            None => include_bytes!(concat!(env!("OUT_DIR"), "/metadata.wasm")).to_vec(),
        };
        Self::from_bytes(&bytes)
    }
    fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > MODULE_BYTES || !bytes.starts_with(b"\0asm") {
            return Err("adapter_invalid");
        }
        let mut config = wasmi::Config::default();
        config.consume_fuel(true).allow_start_fn(false).set_max_recursion_depth(64);
        config.set_min_stack_height(1024).set_max_stack_height(64 * 1024);
        let engine = Engine::new(&config);
        let module = Module::new(&engine, bytes).map_err(|_| "adapter_invalid")?;
        if module.imports().next().is_some() {
            return Err("adapter_imports_forbidden");
        }
        let adapter = Self {
            engine,
            module,
            digest: format!("sha256:{:x}", Sha256::digest(bytes)),
        };
        // Reject bad exports and oversized initial allocations at installation time.
        adapter.run()?;
        Ok(adapter)
    }
    pub(crate) fn run(&self) -> Result<Run, &'static str> {
        let limits = StoreLimitsBuilder::new()
            .memory_size(65_536)
            .memories(1)
            .tables(0)
            .table_elements(0)
            .instances(1)
            .trap_on_grow_failure(true)
            .build();
        let mut store = Store::new(&self.engine, limits);
        store.limiter(|limits| limits);
        store.set_fuel(FUEL).map_err(|_| "adapter_unavailable")?;
        let instance = Linker::new(&self.engine)
            .instantiate_and_start(&mut store, &self.module)
            .map_err(|_| "adapter_invalid")?;
        let memory = instance.get_memory(&store, "memory").ok_or("adapter_invalid")?;
        let classify = instance
            .get_typed_func::<(i32, i32, i32, i32), i32>(&store, "classify")
            .map_err(|_| "adapter_invalid")?;
        Ok(Run { store, memory, classify, failed: false })
    }
}
impl Run {
    pub(crate) fn classify(
        &mut self,
        name: &str,
        kind: &str,
        required: Option<bool>,
    ) -> Result<&'static str, &'static str> {
        if self.failed {
            return Err("adapter_failed");
        }
        let result = self.classify_inner(name, kind, required);
        self.failed = result.is_err();
        result
    }
    fn classify_inner(
        &mut self,
        name: &str,
        kind: &str,
        required: Option<bool>,
    ) -> Result<&'static str, &'static str> {
        if name.len() > 128 {
            return Err("adapter_input_invalid");
        }
        self.memory
            .write(&mut self.store, 0, name.as_bytes())
            .map_err(|_| "adapter_failed")?;
        let kind = match kind {
            "string" => 1,
            "integer" => 2,
            "boolean" => 3,
            "reference" => 4,
            _ => 0,
        };
        let role = self
            .classify
            .call(
                &mut self.store,
                (0, name.len() as i32, kind, required.map(i32::from).unwrap_or(-1)),
            )
            .map_err(|_| "adapter_failed")?;
        match role {
            0 => Ok("unknown"),
            1 => Ok("lifecycle_state"),
            2 => Ok("assignee"),
            3 => Ok("assignment_group"),
            4 => Ok("active"),
            5 => Ok("last_updated"),
            _ => Err("adapter_output_invalid"),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn adapter(body: &str) -> Adapter {
        Adapter::from_bytes(&wat::parse_str(format!(
            "(module (memory (export \"memory\") 1) (func (export \"classify\") (param i32 i32 i32 i32) (result i32) {body}))"
        )).unwrap()).unwrap()
    }
    #[test]
    fn bundled_adapter_maps_metadata_and_unknowns() {
        let adapter = Adapter::load(None).unwrap();
        let mut run = adapter.run().unwrap();
        for (name, expected) in [
            ("state", "lifecycle_state"),
            ("assigned_to", "assignee"),
            ("assignment_group", "assignment_group"),
            ("active", "active"),
            ("sys_updated_on", "last_updated"),
            ("private_field", "unknown"),
        ] {
            assert_eq!(run.classify(name, "string", None), Ok(expected));
        }
    }
    #[test]
    fn replacement_adapter_changes_mapping() {
        assert_eq!(
            adapter("i32.const 2").run().unwrap().classify("state", "string", None),
            Ok("assignee")
        );
    }
    #[test]
    fn infinite_loop_exhausts_fuel_and_poisoned_run_stops() {
        let mut run = adapter("(loop $spin (br $spin)) i32.const 0").run().unwrap();
        assert_eq!(run.classify("state", "integer", None), Err("adapter_failed"));
        assert_eq!(run.classify("active", "boolean", None), Err("adapter_failed"));
    }
    #[test]
    fn growth_traps_and_output_is_validated() {
        assert_eq!(
            adapter("i32.const 1 memory.grow")
                .run()
                .unwrap()
                .classify("state", "integer", None),
            Err("adapter_failed")
        );
        assert_eq!(
            adapter("i32.const 99").run().unwrap().classify("state", "integer", None),
            Err("adapter_output_invalid")
        );
    }
    #[test]
    fn recursive_guest_is_bounded() {
        let bytes = wat::parse_str("(module (memory (export \"memory\") 1) (func $recurse (export \"classify\") (param i32 i32 i32 i32) (result i32) i32.const 0 i32.const 0 i32.const 0 i32.const 0 call $recurse))").unwrap();
        let mut run = Adapter::from_bytes(&bytes).unwrap().run().unwrap();
        assert_eq!(run.classify("state", "string", None), Err("adapter_failed"));
    }
    #[test]
    fn run_state_does_not_leak_to_next_discovery() {
        let bytes = wat::parse_str("(module (memory (export \"memory\") 1) (global $count (mut i32) (i32.const 0)) (func (export \"classify\") (param i32 i32 i32 i32) (result i32) global.get $count i32.const 1 i32.add global.set $count global.get $count))").unwrap();
        let adapter = Adapter::from_bytes(&bytes).unwrap();
        let mut run = adapter.run().unwrap();
        assert_eq!(run.classify("a", "string", None), Ok("lifecycle_state"));
        assert_eq!(run.classify("a", "string", None), Ok("assignee"));
        assert_eq!(adapter.run().unwrap().classify("a", "string", None), Ok("lifecycle_state"));
    }
    #[test]
    fn out_of_bounds_access_traps_and_next_run_is_fresh() {
        let adapter =
            adapter("local.get 2 if (result i32) i32.const 65536 i32.load else i32.const 1 end");
        let mut run = adapter.run().unwrap();
        assert_eq!(run.classify("a", "string", None), Err("adapter_failed"));
        assert_eq!(run.classify("a", "unknown", None), Err("adapter_failed"));
        assert_eq!(adapter.run().unwrap().classify("a", "unknown", None), Ok("lifecycle_state"));
    }
    #[test]
    fn rejects_invalid_exports_and_module_size() {
        for source in ["(module (memory (export \"memory\") 1))", "(module (memory (export \"memory\") 1) (func (export \"classify\") (result i32) i32.const 0))"] {
            assert!(Adapter::from_bytes(&wat::parse_str(source).unwrap()).is_err());
        }
        let mut oversized = vec![0; MODULE_BYTES + 1];
        oversized[..4].copy_from_slice(&[0, 97, 115, 109]);
        assert!(Adapter::from_bytes(&oversized).is_err());
    }
    #[test]
    fn rejects_imports_start_and_oversized_memory() {
        for source in ["(module (import \"wasi_snapshot_preview1\" \"environ_get\" (func)))", "(module (func $start) (start $start))", "(module (memory (export \"memory\") 2) (func (export \"classify\") (param i32 i32 i32 i32) (result i32) i32.const 0))"] {
            assert!(Adapter::from_bytes(&wat::parse_str(source).unwrap()).is_err());
        }
    }
}
